

pub mod manifest;
pub mod reflection;
pub mod templating;
pub mod parsing;



use std::{env, fs};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};
use serde_json;
use serde_derive::{Serialize, Deserialize};
use uuid::Uuid;
use crate::manifest::{Unit, PropertyDefinition, ComponentDefinition, TemplateNodeDefinition, ControlFlowAttributeValueDefinition, ControlFlowRepeatPredicateDeclaration, AttributeValueDefinition, Number, SettingsLiteralValue, SettingsSelectorBlockDefinition, SettingsLiteralBlockDefinition, SettingsValueDefinition};

pub use lazy_static::lazy_static;
use tera::Template;




pub enum PaxContents {
    FilePath(String),
    Inline(String),
}

