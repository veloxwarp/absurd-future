# absurd-future

[![crates.io](https://img.shields.io/crates/v/absurd-future.svg)](https://crates.io/crates/absurd-future)
[![docs.rs](https://docs.rs/absurd-future/badge.svg)](https://docs.rs/absurd-future)

## Stewardship

This repository is an independent continuation of [fpco/absurd-future](https://github.com/fpco/absurd-future), imported with its full Git history. It is maintained by Velox Warp. Original authorship, attribution, and licensing remain preserved.

A future adapter that changes the return type of a future that never resolves (i.e., one that returns `Infallible`) to any other type.

This is useful when you have a task that runs forever (like a background service) but need to use it with an API that expects a specific return type, such as `tokio::task::JoinSet`.

For a detailed explanation of the motivation and the concept of uninhabited types in Rust async code, see the blog post: [How to use Rust's never type (!) to write cleaner async code](https://academy.fpblock.com/blog/rust-never-type-async-code).

## The Problem

Tools like `tokio::task::JoinSet` are great for managing multiple concurrent tasks, but they require all spawned tasks to have the same return type. This can be a problem when you have different kinds of background tasks:

1.  A task that runs forever and never returns: `async fn task_one() -> Infallible`.
2.  A task that runs forever but can fail: `async fn task_two() -> Result<Infallible, Error>`.

These two futures cannot be placed in the same `JoinSet` because their return types differ.

## The Solution: `absurd-future`

This is where `absurd-future` comes in. It's a simple future adapter that takes a future returning an uninhabited type (like `Infallible`) and transforms its type signature to *any other type* you need, without changing its behavior.

This is safe because a value of an uninhabited type can never be constructed. Since the original future can never produce such a value, we can safely claim it produces a value of any other type, because that code path is unreachable.

## Example with `JoinSet`

Here's how to use `absurd_future` to run two tasks with different return types in the same `JoinSet`.

First, we have two tasks. `task_one` never returns (`Infallible`), while `task_two` can return an error (`Result<Infallible>`).

```rust
use std::convert::Infallible;
use std::time::Duration;

async fn task_one() -> Infallible {
    loop {
        println!("Hello from task 1");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
```

```rust
use anyhow::{bail, Result};
use std::convert::Infallible;
use std::time::Duration;

async fn task_two() -> Result<Infallible> {
    let mut counter = 1;
    loop {
        println!("Hello from task 2");
        counter += 1;
        tokio::time::sleep(Duration::from_secs(1)).await;
        if counter >= 3 {
            bail!("Counter is >= 3")
        }
    }
}
```

To run them in the same `JoinSet`, we wrap `task_one` with `absurd_future`. The compiler infers the target type (`Result<Infallible>`) from the `JoinSet`.

```rust
use absurd_future::absurd_future;
use tokio::task::JoinSet;
use anyhow::{bail, Result};
use std::convert::Infallible;

// ... task_one and task_two definitions from above ...

async fn main_inner() -> Result<()> {
    let mut join_set = JoinSet::<Result<Infallible>>::new();

    // Spawn task_two directly.
    join_set.spawn(task_two());

    // This would not compile due to a type mismatch:
    // join_set.spawn(task_one());

    // Wrap task_one with absurd_future to change its return type
    // from Infallible to Result<Infallible>, matching the JoinSet.
    join_set.spawn(absurd_future(task_one()));

    // Now, wait for a task to complete.
    match join_set.join_next().await {
        Some(result) => match result {
            Ok(res) => match res {
                // This branch is impossible, as Infallible can't be created.
                Ok(_res) => bail!("Impossible: Infallible witnessed!"),
                // This is the expected path: task_two fails.
                Err(e) => {
                    join_set.abort_all();
                    bail!("Task exited with {e}")
                }
            },
            Err(e) => { // Task panicked
                join_set.abort_all();
                bail!("Task exited with {e}")
            }
        },
        None => { // No tasks were in the set
            bail!("No tasks found in task set")
        }
    }
}
```

In `main_inner`, we create our `JoinSet`. We can spawn `task_two` without any issues. However, if we
tried to spawn `task_one`, we'd get a compile error because `Infallible` does not match
`Result<Infallible>`.

By wrapping `task_one` with `absurd_future(task_one())`, we adapt its return type. The compiler infers
that we want to change it from `Infallible` to `Result<Infallible>`, and now it can be added to the
`JoinSet`.

When we `join_next()`, we only expect to see the error from `task_two`. The `Ok(_res)` arm is logically
unreachable, as `task_one` will never return and `task_two` only returns an `Err`.

## License

This project is licensed under the MIT license.
