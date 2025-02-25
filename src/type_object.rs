// Copyright (c) 2017-present PyO3 Project and Contributors
//! Python type object information

use crate::types::{PyAny, PyType};
use crate::{ffi, AsPyPointer, PyNativeType, Python};

/// `T: PyLayout<U>` represents that `T` is a concrete representation of `U` in the Python heap.
/// E.g., `PyCell` is a concrete representation of all `pyclass`es, and `ffi::PyObject`
/// is of `PyAny`.
///
/// This trait is intended to be used internally.
///
/// # Safety
///
/// This trait must only be implemented for types which represent valid layouts of Python objects.
pub unsafe trait PyLayout<T> {}

/// `T: PySizedLayout<U>` represents that `T` is not a instance of
/// [`PyVarObject`](https://docs.python.org/3.8/c-api/structures.html?highlight=pyvarobject#c.PyVarObject).
/// In addition, that `T` is a concrete representation of `U`.
pub trait PySizedLayout<T>: PyLayout<T> + Sized {}

/// Python type information.
/// All Python native types (e.g., `PyDict`) and `#[pyclass]` structs implement this trait.
///
/// This trait is marked unsafe because:
///  - specifying the incorrect layout can lead to memory errors
///  - the return value of type_object must always point to the same PyTypeObject instance
///
/// It is safely implemented by the `pyclass` macro.
///
/// # Safety
///
/// Implementations must provide an implementation for `type_object_raw` which infallibly produces a
/// non-null pointer to the corresponding Python type object.
pub unsafe trait PyTypeInfo: Sized {
    /// Class name.
    const NAME: &'static str;

    /// Module name, if any.
    const MODULE: Option<&'static str>;

    /// Utility type to make Py::as_ref work.
    type AsRefTarget: PyNativeType;

    /// Returns the PyTypeObject instance for this type.
    fn type_object_raw(py: Python<'_>) -> *mut ffi::PyTypeObject;

    /// Returns the safe abstraction over the type object.
    fn type_object(py: Python<'_>) -> &PyType {
        unsafe { py.from_borrowed_ptr(Self::type_object_raw(py) as _) }
    }

    /// Checks if `object` is an instance of this type or a subclass of this type.
    fn is_type_of(object: &PyAny) -> bool {
        unsafe { ffi::PyObject_TypeCheck(object.as_ptr(), Self::type_object_raw(object.py())) != 0 }
    }

    /// Checks if `object` is an instance of this type.
    fn is_exact_type_of(object: &PyAny) -> bool {
        unsafe { ffi::Py_TYPE(object.as_ptr()) == Self::type_object_raw(object.py()) }
    }
}

/// Legacy trait which previously held the `type_object` method now found on `PyTypeInfo`.
///
/// # Safety
///
/// This trait used to have stringent safety requirements, but they are now irrelevant as it is deprecated.
#[deprecated(
    since = "0.17.0",
    note = "PyTypeObject::type_object was moved to PyTypeInfo::type_object"
)]
pub unsafe trait PyTypeObject: PyTypeInfo {}

#[allow(deprecated)]
unsafe impl<T: PyTypeInfo> PyTypeObject for T {}

#[inline]
pub(crate) unsafe fn get_tp_alloc(tp: *mut ffi::PyTypeObject) -> Option<ffi::allocfunc> {
    #[cfg(not(Py_LIMITED_API))]
    {
        (*tp).tp_alloc
    }

    #[cfg(Py_LIMITED_API)]
    {
        let ptr = ffi::PyType_GetSlot(tp, ffi::Py_tp_alloc);
        std::mem::transmute(ptr)
    }
}

#[inline]
pub(crate) unsafe fn get_tp_free(tp: *mut ffi::PyTypeObject) -> ffi::freefunc {
    #[cfg(not(Py_LIMITED_API))]
    {
        (*tp).tp_free.unwrap()
    }

    #[cfg(Py_LIMITED_API)]
    {
        let ptr = ffi::PyType_GetSlot(tp, ffi::Py_tp_free);
        debug_assert_ne!(ptr, std::ptr::null_mut());
        std::mem::transmute(ptr)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[allow(deprecated)]
    fn test_deprecated_type_object() {
        // Even though PyTypeObject is deprecated, simple usages of it as a trait bound should continue to work.
        use super::PyTypeObject;
        use crate::types::{PyList, PyType};
        use crate::Python;

        fn get_type_object<T: PyTypeObject>(py: Python<'_>) -> &PyType {
            T::type_object(py)
        }

        Python::with_gil(|py| {
            assert!(get_type_object::<PyList>(py).is(<PyList as crate::PyTypeInfo>::type_object(py)))
        });
    }
}
