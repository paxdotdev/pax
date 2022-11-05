#[macro_use]
extern crate pest_derive;
extern crate core;

use tokio::net::{TcpListener, TcpStream};

use tokio::task::yield_now;
use tokio::{select, task};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{Sender, Receiver, UnboundedReceiver};
use tokio_stream::wrappers::{ReceiverStream};

use std::io::{Error};
use std::task::{Poll, Context};
use std::{fs, thread::{Thread, self}, time::Duration};
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Write;

use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;

use clap::{App, AppSettings, Arg};

use futures::prelude::*;

use serde::Serialize;

// use crate::parser::message::*;
use serde_json::{Value, json};
use tera::Tera;

use tokio::sync::oneshot;
use tokio_serde::SymmetricallyFramed;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
use tokio_serde::formats::*;
// use pax_compiler::PaxManifest;


use uuid::Uuid;


use crate::manifest::PaxManifest;

// use crate::{PaxManifest, press_template_codegen_properties_coproduct_lib, press_template_codegen_cartridge_lib, TemplateArgsCodegenCartridgeLib, TemplateArgsCodegenPropertiesCoproductLib};




