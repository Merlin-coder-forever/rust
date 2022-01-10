//! C API extensions to experiment with eager execution of kernels.
//!
//! WARNING: The underlying C-API for the eager execution is not guaranteed to be
//! stable and can be changed without notice, which could result in breaking.

mod context;
pub use context::*;

mod readonly_tensor;
pub use readonly_tensor::*;

mod tensor_handle;
pub use tensor_handle::*;

mod op;

pub use op::raw_ops;

use crate::{Result, Tensor, TensorType};

impl<T: TensorType> Tensor<T> {
    /// Convert a Tensor to a readonly Tensor for use in eager execution.
    pub fn freeze(self) -> ReadonlyTensor<T> {
        ReadonlyTensor {
            inner: self.inner,
            dims: self.dims,
        }
    }

    /// Convert a Tensor to a TensorHandle for use in eager execution.
    pub fn into_handle<'a>(self, ctx: &'a Context) -> Result<TensorHandle<'a>> {
        self.freeze().to_handle(ctx)
    }
}

/// A helper trait to convert a Tensor or some other types into a TensorHandle
/// for use in eager execution.
pub trait ToTensorHandle<'a> {
    /// Convert a Tensor or values into a new TensorHandle.
    fn to_handle(&self, ctx: &'a Context) -> Result<TensorHandle<'a>>;
}

impl<'a, T> ToTensorHandle<'a> for ReadonlyTensor<T>
where
    T: TensorType,
{
    fn to_handle(&self, ctx: &'a Context) -> Result<TensorHandle<'a>> {
        TensorHandle::new(ctx, self)
    }
}

impl<'a> ToTensorHandle<'a> for TensorHandle<'a> {
    fn to_handle(&self, _: &'a Context) -> Result<TensorHandle<'a>> {
        self.copy_sharing_tensor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eager::{Context, ContextOptions};
    use crate::Tensor;

    #[test]
    fn tensor_to_handle() {
        let ctx = Context::new(ContextOptions::new()).unwrap();

        // Create a read-only tensor.
        let tensor = Tensor::<i32>::new(&[1, 2, 3, 4]).freeze();
        let handle = tensor.to_handle(&ctx).unwrap();
        assert_eq!(handle.num_dims().unwrap(), 4);
        assert_eq!(handle.dim(0).unwrap(), 1);
        assert_eq!(handle.dim(1).unwrap(), 2);
        assert_eq!(handle.dim(2).unwrap(), 3);
        assert_eq!(handle.dim(3).unwrap(), 4);

        // Create a TensorHandle directly from a Tensor.
        // It takes ownership of the Tensor.
        let tensor = Tensor::<i32>::new(&[1, 2, 3, 4]);
        let handle = tensor.into_handle(&ctx).unwrap();
        assert_eq!(handle.num_dims().unwrap(), 4);
        assert_eq!(handle.dim(0).unwrap(), 1);
        assert_eq!(handle.dim(1).unwrap(), 2);
        assert_eq!(handle.dim(2).unwrap(), 3);
        assert_eq!(handle.dim(3).unwrap(), 4);
    }

    #[test]
    fn handle_to_handle() {
        let ctx = Context::new(ContextOptions::new()).unwrap();
        let tensor = Tensor::<i32>::new(&[1, 2, 3, 4]).freeze();
        let handle = tensor.to_handle(&ctx).unwrap();
        let handle2 = handle.to_handle(&ctx).unwrap();
        let tensor2 = handle2.resolve::<i32>().unwrap();
        assert_eq!(tensor, tensor2);
    }

    #[test]
    fn handle_to_tensor_unsafe() {
        let ctx = Context::new(ContextOptions::new()).unwrap();
        let tensor = Tensor::from(0i32).freeze();
        let h = tensor.to_handle(&ctx).unwrap();

        // Getting multiple times should return the same Tensor.
        let t0 = h.resolve::<i32>().unwrap();
        assert_eq!(t0[0], 0i32);

        // Manipulating the Tensor will affect the Tensor that shares underlying buffer.
        {
            let t1 = h.resolve::<i32>().unwrap();

            // Convert back from a TensorHandle to a Tensor.
            let mut t1 = unsafe { t1.into_tensor() };
            t1[0] = 5;
        }

        // Check that t0 shares the same underlying buffer with t1.
        // This is why we need to use unsafe.
        assert_eq!(t0[0], 5);
    }
}