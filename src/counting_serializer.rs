use serde::{ser, Serialize};

/// This is a serializer that counts the elements in a sequence
pub(crate) struct CountingSerializer {
    pub(crate) num_elements: usize,
}

/// This is a dummy error type for CountingSerializer. Currently we don't expect the serializer
/// to fail, so it's empty for now
#[derive(Debug)]
pub(crate) struct CountingSerializerError;

impl std::fmt::Display for CountingSerializerError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}
impl std::error::Error for CountingSerializerError {
    fn description(&self) -> &str {
        ""
    }
    fn cause(&self) -> Option<&dyn std::error::Error> {
        None
    }
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
impl ser::Error for CountingSerializerError {
    fn custom<T>(_msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        CountingSerializerError
    }
}

impl<'a> ser::Serializer for &'a mut CountingSerializer {
    type Ok = ();
    type Error = CountingSerializerError;

    type SerializeSeq = Self;
    type SerializeTuple = ser::Impossible<(), Self::Error>;
    type SerializeTupleStruct = ser::Impossible<(), Self::Error>;
    type SerializeTupleVariant = ser::Impossible<(), Self::Error>;
    type SerializeMap = ser::Impossible<(), Self::Error>;
    type SerializeStruct = ser::Impossible<(), Self::Error>;
    type SerializeStructVariant = ser::Impossible<(), Self::Error>;

    fn serialize_bool(self, _v: bool) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i8(self, _v: i8) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i16(self, _v: i16) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i32(self, _v: i32) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i64(self, _v: i64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u8(self, _v: u8) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u16(self, _v: u16) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u32(self, _v: u32) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u64(self, _v: u64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_f32(self, _v: f32) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_f64(self, _v: f64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_char(self, _v: char) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_str(self, _v: &str) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_none(self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_some<T>(self, _value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    fn serialize_unit(self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!()
    }
}

impl<'a> ser::SerializeSeq for &'a mut CountingSerializer {
    type Ok = ();
    type Error = CountingSerializerError;

    fn serialize_element<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.num_elements += 1;
        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}
