# Calling async Python functions in Rust code

In the previous chapter, we learnt how to call Python functions in Rust code. Now, we will look at two ways to call Python async functions from Rust code.

## Async Functions

Before we start, let us take a look at the following snippet.

```python
async def fx():
    return "hello, world"
```

Above, we have defined an async function. How we send it to Rust code will make a difference.

In python, even an async function is treated like a regular function until called.
e.g.

```python
print(type fx) # this will output the type function
print(type fx()) # this will output the type of a coroutine
```

The native object `PyAny` can be used to call the Python functions in Rust code.

Now, let us take a look at how will we share the Python function in our rust code.

```rust

use pyo3::prelude::*;

#[pyclass]
struct SomeObject {}

#[pymethods]
impl SomeObject {
    #[new]
    fn new() -> Self {
        Self {}
    }

    fn start(&self, python_function: &PyAny) {
        // where python_function is the function added from python
    }
}

#[pymodule]
pub fn some_module(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Server>()?;
    pyo3_asyncio::try_init(py);
    Ok(())
}

```

Now, to start it in python, we will do something like the following code:

```python

import some_module

some_object = some_module.SomeObject()

async def fx():
    return "hello world"

some_object.start() # what we pass here, will determine the code we write in our rust start
```

The `start` function shows how we will be able to send our Python code to rust.

Now, we have two choices. Either we can pass the Python function as the following:

```python
some_object.start(fx)
```

or, we can pass an awaitable coroutine

```python
some_object.start(fx())
```

In both cases, we will have to take the help of an external crate called [pyo3_asyncio](https://crates.io/crates/pyo3-asyncio)

Now, let us say we take the first approach of passing a function.

```rust
use pyo3::prelude::*;
use pyo3::prelude::*;

#[pyclass]
struct SomeObject {}

#[pymethods]
impl SomeObject {
    #[new]
    fn new() -> Self {
        Self {}
    }

    fn start(&self, python_function: &PyAny) {
        // where python_function is the function added from python
        Python::with_gil(|py| {
            // here we are converting the function to a coroutine by calling it
            let coro = handler.as_ref(py).call0().unwrap();
            // here we are converting the coroutine to a Future
            // converting into a Future is mandatory to be awaited in rust
            let f = pyo3_asyncio::into_future(&coro).unwrap()

            pyo3_asyncio::async_std::run_until_complete(py, async move {
                f.await?; // here the async function is called
                Ok(())
            });
        });
        // now we will need to convert the function to a coroutine
    }
}

#[pymodule]
pub fn some_module(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Server>()?;
    pyo3_asyncio::try_init(py);
    Ok(())
}

```

Now, if we had passed a pre-made python coroutine , we wouldn't have to write the following code snippet:

```rust
let coro = handler.as_ref(py).call0().unwrap();
```

As, we are already passing a pre-made coroutine. So, the code snippet would look like:

```rust
use pyo3::prelude::*;
use pyo3::prelude::*;

#[pyclass]
struct SomeObject {}

#[pymethods]
impl SomeObject {
    #[new]
    fn new() -> Self {
        Self {}
    }

    fn start(&self, python_function: &PyAny) {
        // where python_function is the function added from python
        Python::with_gil(|py| {
            // here we are converting the coroutine to a Future
            // converting into a Future is mandatory to be awaited in rust
            let f = pyo3_asyncio::into_future(&coro).unwrap()

            pyo3_asyncio::async_std::run_until_complete(py, async move {
                f.await?; // here the async function is called
                Ok(())
            });
        });
        // now we will need to convert the function to a coroutine
    }
}

#[pymodule]
pub fn some_module(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Server>()?;
    pyo3_asyncio::try_init(py);
    Ok(())
}

```

Now, a _major_ reason where you would want to choose one approach over another is that a pre made coroutine cannot be awaited more than once.

e.g. if you are planning to store one coroutine and call it multiple times for some reason, it won't be possible and you would be getting the error

```rust
RuntimeError: cannot reuse already awaited coroutine
```

So, if you are planning to store one function and reuse it again. It is sensible to pass it as a function, otherwise you can also pass it as a coroutine.

You can read more about the crate [here](https://crates.io/crates/pyo3-asyncio).
