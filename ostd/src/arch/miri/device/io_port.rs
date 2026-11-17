// SPDX-License-Identifier: MPL-2.0

//! I/O port access.

use core::marker::PhantomData;

pub struct WriteOnlyAccess;
pub struct ReadWriteAccess;

pub trait IoPortWriteAccess {}
pub trait IoPortReadAccess {}

impl IoPortWriteAccess for WriteOnlyAccess {}
impl IoPortWriteAccess for ReadWriteAccess {}
impl IoPortReadAccess for ReadWriteAccess {}

pub trait PortRead: Sized {
    unsafe fn read_from_port(port: u16) -> Self;
}

pub trait PortWrite: Sized {
    unsafe fn write_to_port(port: u16, value: Self);
}

// Port I/O goes to KernMiri's port bus, which decides what each port does.
macro_rules! impl_port_access {
    ($($ty:ty),*) => {$(
        impl PortRead for $ty {
            unsafe fn read_from_port(port: u16) -> Self {
                // SAFETY: A port bus access does not touch the kernel's memory.
                unsafe { super::super::kern_miri_port_read(port, size_of::<$ty>()) as $ty }
            }
        }

        impl PortWrite for $ty {
            unsafe fn write_to_port(port: u16, value: Self) {
                // SAFETY: A port bus access does not touch the kernel's memory.
                unsafe { super::super::kern_miri_port_write(port, size_of::<$ty>(), value as u32) }
            }
        }
    )*};
}

impl_port_access!(u8, u16, u32);

/// An I/O port, representing a specific address in the I/O address of x86.
///
/// The following code shows and example to read and write u32 value to an I/O port:
///
/// ```rust
/// static PORT: IoPort<u32, ReadWriteAccess> = unsafe { IoPort::new(0x12) };
///
/// fn port_value_increase(){
///     PORT.write(PORT.read() + 1)
/// }
/// ```
///
pub struct IoPort<T, A> {
    port: u16,
    value_marker: PhantomData<T>,
    access_marker: PhantomData<A>,
}

impl<T, A> IoPort<T, A> {
    /// Create an I/O port.
    ///
    /// # Safety
    ///
    /// This function is marked unsafe as creating an I/O port is considered
    /// a privileged operation.
    pub const unsafe fn new(port: u16) -> Self {
        Self {
            port,
            value_marker: PhantomData,
            access_marker: PhantomData,
        }
    }
}

impl<T: PortRead, A: IoPortReadAccess> IoPort<T, A> {
    /// Reads from the I/O port
    #[inline]
    pub fn read(&self) -> T {
        unsafe { PortRead::read_from_port(self.port) }
    }
}

impl<T: PortWrite, A: IoPortWriteAccess> IoPort<T, A> {
    /// Writes to the I/O port
    #[inline]
    pub fn write(&self, value: T) {
        unsafe { PortWrite::write_to_port(self.port, value) }
    }
}
