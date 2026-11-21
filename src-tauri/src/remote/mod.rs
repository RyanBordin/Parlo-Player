//! Remote control on the network (spec 12.8): the computer serves
//! (`server`), the phone finds and controls it (`client`). The messages and
//! the sign in are shared in `parlo_core::remote`.

mod client;
#[cfg(desktop)]
mod server;

pub use client::{RemoteClient, RemoteDevice, RemoteError};
pub use parlo_core::remote::*;
#[cfg(desktop)]
pub use server::{start, stop};
