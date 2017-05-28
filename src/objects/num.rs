// Copyright (c) 2017-present PyO3 Project and Contributors
//
// based on Daniel Grunwald's https://github.com/dgrunwald/rust-cpython

extern crate num_traits;

use self::num_traits::cast::cast;
use std::os::raw::{c_long, c_double};

use ::{Py, PyPtr};
use ffi;
use super::exc;
use super::PyObject;
use typeob::PyTypeInfo;
use python::{Python, PythonToken, ToPythonPointer, PythonObjectWithToken, Token};
use err::{PyResult, PyErr};
use conversion::{ToPyObject, FromPyObject};

/// Represents a Python `int` object.
///
/// You can usually avoid directly working with this type
/// by using [ToPyObject](trait.ToPyObject.html)
/// and [extract](struct.PyObject.html#method.extract)
/// with the primitive Rust integer types.
pub struct PyLong(PythonToken<PyLong>);
pyobject_newtype!(PyLong, PyLong_Check, PyLong_Type);

/// Represents a Python `float` object.
///
/// You can usually avoid directly working with this type
/// by using [ToPyObject](trait.ToPyObject.html)
/// and [extract](struct.PyObject.html#method.extract)
/// with `f32`/`f64`.
pub struct PyFloat(PythonToken<PyFloat>);
pyobject_newtype!(PyFloat, PyFloat_Check, PyFloat_Type);


impl PyFloat {
    /// Creates a new Python `float` object.
    pub fn new(_py: Token, val: c_double) -> PyPtr<PyFloat> {
        unsafe {
            PyPtr::from_owned_ptr_or_panic(ffi::PyFloat_FromDouble(val))
        }
    }

    /// Gets the value of this float.
    pub fn value(&self) -> c_double {
        unsafe { ffi::PyFloat_AsDouble(self.as_ptr()) }
    }
}


macro_rules! int_fits_c_long(
    ($rust_type:ty) => (
        impl ToPyObject for $rust_type {
            fn to_object(&self, py: Token) -> PyPtr<PyObject> {
                unsafe {
                    PyPtr::from_owned_ptr_or_panic(ffi::PyLong_FromLong(*self as c_long))
                }
            }
        }

        pyobject_extract!(obj to $rust_type => {
            let val = unsafe { ffi::PyLong_AsLong(obj.as_ptr()) };
            if val == -1 && PyErr::occurred(obj.token()) {
                return Err(PyErr::fetch(obj.token()));
            }
            match cast::<c_long, $rust_type>(val) {
                Some(v) => Ok(v),
                None => Err(overflow_error(obj.token()))
            }
        });
    )
);


macro_rules! int_fits_larger_int(
    ($rust_type:ty, $larger_type:ty) => (
        impl ToPyObject for $rust_type {
            #[inline]
            fn to_object(&self, py: Token) -> PyPtr<PyObject> {
                (*self as $larger_type).to_object(py)
            }
        }

        pyobject_extract!(obj to $rust_type => {
            let py = obj.token();
            let val = try!(obj.extract::<$larger_type>());
            match cast::<$larger_type, $rust_type>(val) {
                Some(v) => Ok(v),
                None => Err(overflow_error(py))
            }
        });
    )
);


fn err_if_invalid_value<'p, T: PartialEq>
    (py: Token, invalid_value: T, actual_value: T) -> PyResult<T>
{
    if actual_value == invalid_value && PyErr::occurred(py) {
        Err(PyErr::fetch(py))
    } else {
        Ok(actual_value)
    }
}

macro_rules! int_convert_u64_or_i64 (
    ($rust_type:ty, $pylong_from_ll_or_ull:expr, $pylong_as_ull_or_ull:expr) => (
        impl ToPyObject for $rust_type {

            fn to_object(&self, py: Token) -> PyPtr<PyObject> {
                unsafe {
                    PyPtr::from_owned_ptr_or_panic($pylong_from_ll_or_ull(*self))
                }
            }
        }

        impl<'source> FromPyObject<'source> for $rust_type {
            fn extract<S>(py: &'source Py<'source, S>) -> PyResult<$rust_type>
                where S: PyTypeInfo
            {
                let ptr = py.as_ptr();
                unsafe {
                    if ffi::PyLong_Check(ptr) != 0 {
                        err_if_invalid_value(py.token(), !0, $pylong_as_ull_or_ull(ptr))
                    } else {
                        let num = ffi::PyNumber_Long(ptr);
                        if num.is_null() {
                            Err(PyErr::fetch(py.token()))
                        } else {
                            err_if_invalid_value(py.token(), !0, $pylong_as_ull_or_ull(num))
                        }
                    }
                }
            }
        }
    )
);


int_fits_c_long!(i8);
int_fits_c_long!(u8);
int_fits_c_long!(i16);
int_fits_c_long!(u16);
int_fits_c_long!(i32);

// If c_long is 64-bits, we can use more types with int_fits_c_long!:
#[cfg(all(target_pointer_width="64", not(target_os="windows")))]
int_fits_c_long!(u32);
#[cfg(any(target_pointer_width="32", target_os="windows"))]
int_fits_larger_int!(u32, u64);

#[cfg(all(target_pointer_width="64", not(target_os="windows")))]
int_fits_c_long!(i64);

// manual implementation for i64 on systems with 32-bit long
#[cfg(any(target_pointer_width="32", target_os="windows"))]
int_convert_u64_or_i64!(i64, ffi::PyLong_FromLongLong, ffi::PyLong_AsLongLong);

#[cfg(all(target_pointer_width="64", not(target_os="windows")))]
int_fits_c_long!(isize);
#[cfg(any(target_pointer_width="32", target_os="windows"))]
int_fits_larger_int!(isize, i64);

int_fits_larger_int!(usize, u64);

// u64 has a manual implementation as it never fits into signed long
int_convert_u64_or_i64!(u64, ffi::PyLong_FromUnsignedLongLong, ffi::PyLong_AsUnsignedLongLong);

impl ToPyObject for f64 {
    fn to_object(&self, py: Token) -> PyPtr<PyObject> {
        PyFloat::new(py, *self).into_object()
    }
}

pyobject_extract!(obj to f64 => {
    let v = unsafe { ffi::PyFloat_AsDouble(obj.as_ptr()) };
    if v == -1.0 && PyErr::occurred(obj.token()) {
        Err(PyErr::fetch(obj.token()))
    } else {
        Ok(v)
    }
});

fn overflow_error(py: Token) -> PyErr {
    PyErr::new_lazy_init(py.get_type::<exc::OverflowError>(), None)
}

impl ToPyObject for f32 {
    fn to_object(&self, py: Token) -> PyPtr<PyObject> {
        PyFloat::new(py, *self as f64).into_object()
    }
}

pyobject_extract!(obj to f32 => {
    Ok(try!(obj.extract::<f64>()) as f32)
});

#[cfg(test)]
mod test {
    use std;
    use python::Python;
    use conversion::ToPyObject;

    macro_rules! num_to_py_object_and_back (
        ($func_name:ident, $t1:ty, $t2:ty) => (
            #[test]
            fn $func_name() {
                let gil = Python::acquire_gil();
                let py = gil.python();
                let val = 123 as $t1;
                let obj = val.to_object(py).into_object();
                assert_eq!(obj.extract::<$t2>(py).unwrap(), val as $t2);
            }
        )
    );

    num_to_py_object_and_back!(to_from_f64, f64, f64);
    num_to_py_object_and_back!(to_from_f32, f32, f32);
    num_to_py_object_and_back!(to_from_i8,   i8,  i8);
    num_to_py_object_and_back!(to_from_u8,   u8,  u8);
    num_to_py_object_and_back!(to_from_i16, i16, i16);
    num_to_py_object_and_back!(to_from_u16, u16, u16);
    num_to_py_object_and_back!(to_from_i32, i32, i32);
    num_to_py_object_and_back!(to_from_u32, u32, u32);
    num_to_py_object_and_back!(to_from_i64, i64, i64);
    num_to_py_object_and_back!(to_from_u64, u64, u64);
    num_to_py_object_and_back!(to_from_isize, isize, isize);
    num_to_py_object_and_back!(to_from_usize, usize, usize);
    num_to_py_object_and_back!(float_to_i32, f64, i32);
    num_to_py_object_and_back!(float_to_u32, f64, u32);
    num_to_py_object_and_back!(float_to_i64, f64, i64);
    num_to_py_object_and_back!(float_to_u64, f64, u64);
    num_to_py_object_and_back!(int_to_float, i32, f64);

    #[test]
    fn test_u32_max() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let v = std::u32::MAX;
        let obj = v.to_py_object(py).into_object();
        assert_eq!(v, obj.extract::<u32>(py).unwrap());
        assert_eq!(v as u64, obj.extract::<u64>(py).unwrap());
        assert!(obj.extract::<i32>(py).is_err());
    }
    
    #[test]
    fn test_i64_max() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let v = std::i64::MAX;
        let obj = v.to_py_object(py).into_object();
        assert_eq!(v, obj.extract::<i64>(py).unwrap());
        assert_eq!(v as u64, obj.extract::<u64>(py).unwrap());
        assert!(obj.extract::<u32>(py).is_err());
    }
    
    #[test]
    fn test_i64_min() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let v = std::i64::MIN;
        let obj = v.to_py_object(py).into_object();
        assert_eq!(v, obj.extract::<i64>(py).unwrap());
        assert!(obj.extract::<i32>(py).is_err());
        assert!(obj.extract::<u64>(py).is_err());
    }
    
    #[test]
    fn test_u64_max() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let v = std::u64::MAX;
        let obj = v.to_py_object(py).into_object();
        println!("{:?}", obj);
        assert_eq!(v, obj.extract::<u64>(py).unwrap());
        assert!(obj.extract::<i64>(py).is_err());
    }
}
