use std::collections::HashMap;

#[derive(Debug)]
pub enum DataType {
    Int(i32),
    Text(Box<[u8]>),
    Array(Box<[DataType]>),
    List(Vec<DataType>),
}

impl From<i32> for DataType {
    #[inline]
    fn from(value: i32) -> Self {
        DataType::Int(value)
    }
}

impl From<String> for DataType {
    #[inline]
    fn from(value: String) -> Self {
        let bytes = value.into_bytes().into_boxed_slice();

        DataType::Text(bytes)
    }
}

impl From<&str> for DataType {
    #[inline]
    fn from(value: &str) -> Self {
        DataType::from(value.to_string())
    }
}

impl<T> From<&[T]> for DataType
where
    T: Clone + Into<Self>,
{
    #[inline]
    fn from(slice: &[T]) -> Self {
        let boxed_array = slice
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<DataType>>()
            .into_boxed_slice();

        DataType::Array(boxed_array)
    }
}

impl<T> From<Vec<T>> for DataType
where
    T: Clone + Into<Self>,
{
    #[inline]
    fn from(value: Vec<T>) -> Self {
        let values = value.iter().cloned().map(Into::into).collect::<Vec<_>>();

        DataType::List(values)
    }
}

pub type DataTypeMap = HashMap<String, DataType>;
