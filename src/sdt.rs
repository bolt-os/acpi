use crate::{size_of_unsized, Rsdp};
use core::{
    fmt,
    mem::size_of,
    ptr::{self, addr_of, Pointee},
};
use libsa::endian::{u32_le, u64_le};

pub mod fadt;
pub mod madt;
pub mod mcfg;
pub mod rhct;

pub use mapped::Mapped;

pub trait Bridge: Copy {
    fn map(&self, phys: usize, size: usize) -> usize;
    fn remap(&self, virt: usize, new_size: usize) -> usize;
    fn unmap(&self, virt: usize);
}

mod mapped {
    use super::{Bridge, Header, Sdt};
    use core::{num::NonZeroUsize, ops::Deref, ptr::NonNull};

    pub struct Mapped<T: ?Sized, B: Bridge> {
        ptr: NonNull<T>,
        bridge: B,
    }

    impl<T: ?Sized, B: Bridge> Drop for Mapped<T, B> {
        fn drop(&mut self) {
            self.bridge.unmap(self.ptr.addr().get())
        }
    }

    impl<T: ?Sized, B: Bridge> Mapped<T, B> {
        pub(crate) fn new(ptr: *const T, bridge: B) -> Mapped<T, B> {
            let ptr = NonNull::new(ptr.cast_mut()).unwrap();
            Mapped { ptr, bridge }
        }

        pub fn into_inner(self) -> *const T {
            let ptr = self.ptr;
            core::mem::forget(self);
            ptr.as_ptr().cast_const()
        }
    }

    impl<B: Bridge> Mapped<Header, B> {
        pub fn clone_header(&self) -> Self {
            let size = self.length as usize;
            let addr = self.bridge.remap(self.ptr.addr().get(), size);
            Self {
                ptr: self.ptr.with_addr(NonZeroUsize::new(addr).unwrap()),
                bridge: self.bridge,
            }
        }

        pub fn map_full<T: ?Sized + Sdt>(self) -> Mapped<T, B> {
            assert!(self.signature == T::SIGNATURE);

            // Map the full size of the table (in bytes) according to the header.
            let addr = self.ptr.addr().get();
            let size = self.length as usize;
            let addr = self.bridge.remap(addr, size);
            let ptr = self.ptr.as_ptr().with_addr(addr);

            // Let the table handle determining the pointer metadata, if any.
            let ptr = unsafe { T::from_header_ptr(ptr) };
            let ptr = NonNull::new(ptr.cast_mut()).unwrap();

            Mapped {
                ptr,
                bridge: self.bridge,
            }
        }
    }

    impl<T: ?Sized, B: Bridge> Deref for Mapped<T, B> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            unsafe { self.ptr.as_ref() }
        }
    }

    impl<T: ?Sized + Sdt, B: Bridge> Clone for Mapped<T, B> {
        fn clone(&self) -> Self {
            let size = self.header().length as usize;
            let addr = self.bridge.remap(self.ptr.addr().get(), size);
            Self {
                ptr: self.ptr.with_addr(NonZeroUsize::new(addr).unwrap()),
                bridge: self.bridge,
            }
        }
    }
}

/// System Description Table
///
/// # Safety
///
/// Types implementing this trait must be valid ACPI Description Tables. Specifically, they
/// must contain a [`Header`] at the beginning, such that casting a pointer to the type to
/// a pointer to `Header` is valid.
pub unsafe trait Sdt {
    /// Table Signature
    ///
    /// Table-specific signature found in the `signature` field of [`Header`]. It is assumed
    /// that a pointer to a`Header` with a matching singature can be cast to a pointer to `Self`.
    const SIGNATURE: Signature;

    /// Create a pointer to a full table from a pointer to its `Header`
    ///
    /// This function handles creating the (potentially fat) pointer to `Self`.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`Header`] with a matching signature, with provenance
    /// over **all** bytes of the entire table, as specified by the `length` field of `Header`.
    unsafe fn from_header_ptr(ptr: *const Header) -> *const Self;

    /// Returns a pointer to the [`Header`] of this table
    ///
    /// The returned pointer retains provenance over all bytes of `Self`.
    ///
    /// # Safety
    ///
    /// `self` must be a valid pointer to a value of `Self`. Due to the safety invariants of
    /// the `Sdt` trait, the returned pointer must necessarily be valid to use.
    unsafe fn header_raw(self: *const Self) -> *const Header {
        self.cast()
    }

    /// Returns a reference to the header of this table
    ///
    /// The returned reference has provenance over **only** the bytes of `Header`, and as such
    /// may **not** be used to get back a reference to `Self`.
    fn header(&self) -> &Header {
        // SAFETY: Since we start with a valid reference we can be sure it is also valid
        //         to create a reference to the header within.
        //         See the safety docs on `.header_raw()`.
        unsafe { &*Self::header_raw(self) }
    }
}

// impl<T: ?Sized + Sdt, B: Bridge> AsRef<Header> for Mapped<T, B> {
//     fn as_ref(&self) -> &Header {
//         self.header()
//     }
// }

#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Signature(pub [u8; 4]);

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0.map(|byte| if byte.is_ascii() { byte } else { b'?' });
        let s = core::str::from_utf8(&bytes).unwrap();
        write!(f, "{s:?}")
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// System Description Table Header
#[repr(C, packed)]
#[derive(Clone, Debug)]
pub struct Header {
    pub signature: Signature,
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: u64,
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

#[derive(Clone, Copy)]
enum RootPtrs {
    Rsdt(*const [u32_le]),
    Xsdt(*const [u64_le]),
}

impl RootPtrs {
    unsafe fn len(self) -> usize {
        match self {
            Self::Rsdt(ptrs) => ptrs.len(),
            Self::Xsdt(ptrs) => ptrs.len(),
        }
    }

    unsafe fn iter<'a, B: Bridge + 'a>(
        &'a self,
        bridge: B,
    ) -> impl Iterator<Item = Mapped<Header, B>> + 'a {
        (0..self.len()).map(move |index| {
            let phys = match self {
                Self::Rsdt(ptrs) => ptrs.get_unchecked(index).read_unaligned().get() as usize,
                Self::Xsdt(ptrs) => ptrs.get_unchecked(index).read_unaligned().get() as usize,
            };
            map_header(phys, bridge)
        })
    }
}

/// Root System Description Table
pub struct RootTable<B: Bridge> {
    pub acpi_revision: u8,
    root_ptrs: RootPtrs,
    bridge: B,
}

unsafe impl<B: Bridge + Send> Send for RootTable<B> {}
unsafe impl<B: Bridge + Sync> Sync for RootTable<B> {}

impl<B: Bridge> RootTable<B> {
    /// Create a new `RootTable` from a pointer to the RSDP
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to an `Rsdp`.
    pub unsafe fn new(rsdp: *const Rsdp, bridge: B) -> RootTable<B> {
        let acpi_revision = (*rsdp).revision;
        let root_ptrs = if acpi_revision < 2 {
            let rsdt = map_table::<RootSdt<u32_le>, _>((*rsdp).rsdt_addr as usize, bridge);
            RootPtrs::Rsdt(addr_of!(rsdt.tables))
        } else {
            let xsdt = map_table::<RootSdt<u64_le>, _>((*rsdp).xsdt_addr as usize, bridge);
            RootPtrs::Xsdt(addr_of!(xsdt.tables))
        };
        Self {
            acpi_revision,
            bridge,
            root_ptrs,
        }
    }

    pub fn all_tables(&self) -> impl Iterator<Item = Mapped<Header, B>> + '_ {
        unsafe {
            self.root_ptrs
                .iter(self.bridge)
                .map(|header| header.clone_header())
        }
    }

    pub fn get_table_by_signature(
        &self,
        signature: Signature,
        index: usize,
    ) -> Option<Mapped<Header, B>> {
        self.all_tables()
            .filter(|header| header.signature == signature)
            .nth(index)
    }

    pub fn get_table<T: ?Sized + Sdt>(&self, index: usize) -> Option<Mapped<T, B>> {
        self.get_table_by_signature(T::SIGNATURE, index)
            .map(Mapped::map_full)
    }
}

#[repr(C, packed)]
struct RootSdt<T: Copy> {
    header: Header,
    tables: [T],
}

unsafe impl Sdt for RootSdt<u32_le> {
    const SIGNATURE: Signature = Signature(*b"RSDT");

    fn header(&self) -> &Header {
        &self.header
    }

    unsafe fn from_header_ptr(header: *const Header) -> *const Self {
        from_header_ptr_slice_of::<u32_le, _>(header)
    }
}

unsafe impl Sdt for RootSdt<u64_le> {
    const SIGNATURE: Signature = Signature(*b"XSDT");

    fn header(&self) -> &Header {
        &self.header
    }

    unsafe fn from_header_ptr(header: *const Header) -> *const Self {
        from_header_ptr_slice_of::<u64_le, _>(header)
    }
}

#[inline]
unsafe fn header_dynamic_size<T>(header: *const Header) -> usize
where
    T: ?Sized + Sdt + Pointee<Metadata = usize>,
{
    debug_assert_eq!((*header).signature, T::SIGNATURE);
    (*header).length as usize - size_of_unsized::<T>()
}

unsafe fn from_header_ptr_slice_of<T, S>(header: *const Header) -> *const S
where
    S: ?Sized + Sdt + Pointee<Metadata = usize>,
{
    let len = header_dynamic_size::<S>(header) / size_of::<T>();
    ptr::from_raw_parts(header, len)
}

unsafe fn map_header<B: Bridge>(phys: usize, bridge: B) -> Mapped<Header, B> {
    let header = ptr::with_exposed_provenance::<Header>(bridge.map(phys, size_of::<Header>()));
    Mapped::new(header, bridge)
}

unsafe fn map_table<T: ?Sized + Sdt, B: Bridge>(phys: usize, bridge: B) -> Mapped<T, B> {
    let header = ptr::with_exposed_provenance::<Header>(bridge.map(phys, size_of::<Header>()));
    let table = T::from_header_ptr(header);
    Mapped::new(table, bridge)
}
