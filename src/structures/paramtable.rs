use crate::cache::buf::Buffer;
use pyo3::prelude::*;
use serde::Serialize;
use std::{collections::HashMap, iter};

/// [`LocationConfig`](crate::definitions::location_configs::LocationConfig)s,
/// items and
/// [`NpcConfig`](crate::definitions::npc_configs::NpcConfig)s can have additional mapping of keys to properties.
#[pyclass]
#[derive(Serialize, Debug, Clone)]
pub struct ParamTable {
    /// Key:Value pairs of additional properties.
    pub params: HashMap<u32, Param>,
}

impl ParamTable {
    /// Constructor for [`ParamTable`]
    pub fn deserialize(buffer: &mut Buffer) -> Self {
        let count = buffer.read_unsigned_byte().into();
        let params = iter::repeat_with(|| Self::sub_deserialize(buffer)).take(count).collect();
        Self { params }
    }

    fn sub_deserialize(buffer: &mut Buffer) -> (u32, Param) {
        let ty = buffer.read_unsigned_byte();

        let key = buffer.read_3_unsigned_bytes();

        let value = match ty {
            0 => Param::Integer(buffer.read_int()),
            1 => Param::String(buffer.read_string()),
            other => unimplemented!("Cannot decode unknown type {}", other),
        };
        (key, value)
    }
}

#[pymethods]
impl ParamTable {
    fn get(&self, id: u32) -> PyResult<Option<&Param>> {
        Ok(self.params.get(&id))
    }
}

/// An additional key:property mapping.
#[derive(Serialize, Debug, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub enum Param {
    /// The integer variant.
    Integer(i32),
    /// The string variant.
    String(String),
}

impl IntoPy<PyObject> for &Param {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Param::Integer(val) => val.into_py(py),
            Param::String(val) => val.into_py(py),
        }
    }
}
