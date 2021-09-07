use crate::{
    apply::ApplyContext,
    difference::{DiffCommandRef, DiffContext, DiffPathElementValue},
    SerdeDiff,
};

use serde::{de, ser::SerializeSeq, Deserialize, Serialize};

use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

macro_rules! array_impls {
    ($($len:tt)+) => {
        $(
            impl<T: $crate::SerdeDiff + serde::Serialize + for<'a> serde::Deserialize<'a>> $crate::SerdeDiff for [T; $len] {
                fn diff<'a, S: serde::ser::SerializeSeq>(
                    &self,
                    ctx: &mut $crate::difference::DiffContext<'a, S>,
                    other: &Self,
                ) -> Result<bool, S::Error> {
                    use $crate::difference::DiffCommandRef;

                    let mut need_exit = false;
                    let mut changed = false;
                    for (idx, (self_item, other_item)) in self.iter().zip(other.iter()).enumerate() {
                        ctx.push_collection_index(idx);
                        if <T as $crate::SerdeDiff>::diff(self_item, ctx, other_item)? {
                            need_exit = true;
                            changed = true;
                        }
                        ctx.pop_path_element()?;
                    }
                    if need_exit {
                        ctx.save_command::<()>(&DiffCommandRef::Exit, true, false)?;
                    }
                    Ok(changed)
                }

                fn apply<'de, A>(
                    &mut self,
                    seq: &mut A,
                    ctx: &mut $crate::apply::ApplyContext,
                ) -> Result<bool, <A as serde::de::SeqAccess<'de>>::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let mut changed = false;
                    while let Some(cmd) = ctx.read_next_command::<A, T>(seq)? {
                        use $crate::difference::DiffCommandValue::*;
                        use $crate::difference::DiffPathElementValue::*;
                        match cmd {
                            // we should not be getting fields when reading collection commands
                            Enter(Field(_)) => {
                                ctx.skip_value(seq)?;
                                break;
                            }
                            Enter(CollectionIndex(idx)) => {
                                if let Some(value_ref) = self.get_mut(idx) {
                                    changed |= <T as $crate::SerdeDiff>::apply(value_ref, seq, ctx)?;
                                } else {
                                    ctx.skip_value(seq)?;
                                }
                            }
                            _ => break,
                        }
                    }
                    Ok(changed)
                }
            }
        )+
    }
}

array_impls! {
    01 02 03 04 05 06 07 08 09 10
    11 12 13 14 15 16 17 18 19 20
    21 22 23 24 25 26 27 28 29 30
    31 32
    40 48 50 56 64 72 96 100 128 160 192 200 224 256 384 512
    768 1024 2048 4096 8192 16384 32768 65536
}

macro_rules! tuple_impls {
    ($($len:expr => ($($n:tt $name:ident)+))+) => {
        $(
            impl<$($name),+> $crate::SerdeDiff for ($($name,)+)
            where
                $($name: $crate::SerdeDiff + serde::Serialize + for<'a> serde::Deserialize<'a>,)+
            {
                fn diff<'a, S: serde::ser::SerializeSeq>(
                    &self,
                    ctx: &mut $crate::difference::DiffContext<'a, S>,
                    other: &Self,
                ) -> Result<bool, S::Error> {
                    let mut changed = false;
                    $(
                        ctx.push_field(stringify!($n));
                        changed |= <$name as $crate::SerdeDiff>::diff(&self.$n, ctx, &other.$n)?;
                        ctx.pop_path_element()?;
                    )+
                    Ok(changed)
                }

                fn apply<'de, A>(
                    &mut self,
                    seq: &mut A,
                    ctx: &mut $crate::apply::ApplyContext,
                ) -> Result<bool, <A as serde::de::SeqAccess<'de>>::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let mut changed = false;
                    while let Some($crate::difference::DiffPathElementValue::Field(element)) = ctx.next_path_element(seq)? {
                        match element.as_ref() {
                            $(
                                stringify!($n) => changed |= <$name as $crate::SerdeDiff>::apply(&mut self.$n, seq, ctx)?,
                            )+
                            _ => ctx.skip_value(seq)?,
                        }
                    }
                    Ok(changed)
                }
            }
        )+
    }
}

tuple_impls! {
    1 => (0 T0)
    2 => (0 T0 1 T1)
    3 => (0 T0 1 T1 2 T2)
    4 => (0 T0 1 T1 2 T2 3 T3)
    5 => (0 T0 1 T1 2 T2 3 T3 4 T4)
    6 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5)
    7 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6)
    8 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7)
    9 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8)
    10 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9)
    11 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10)
    12 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11)
    13 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12)
    14 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13)
    15 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14)
    16 => (0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15)
}
/// Implement SerdeDiff on a "map-like" type such as HashMap.
macro_rules! map_serde_diff {
    ($t:ty, $($extra_traits:path),*) => {
        impl<K, V> SerdeDiff for $t
        where
            K: Serialize + for<'a> Deserialize<'a> $(+ $extra_traits)*, // + Hash + Eq,
            V: SerdeDiff + Serialize + for<'a> Deserialize<'a>,
        {
            fn diff<'a, S: SerializeSeq>(
                &self,
                ctx: &mut $crate::difference::DiffContext<'a, S>,
                other: &Self,
            ) -> Result<bool, S::Error> {
                use $crate::difference::DiffCommandRef;

                let mut changed = false;

                for (key, self_value) in self.iter() {
                    match other.get(key) {
                        Some(other_value) => {
                            let save_closure = |serializer: &mut S| serializer.serialize_element(&DiffCommandRef::EnterKey(key));
                            let mut subctx = ctx.reborrow();
                            subctx.push_field_element(&save_closure);
                            if <V as SerdeDiff>::diff(self_value, &mut subctx, other_value)? {
                                changed = true;
                            }
                        },
                        None => {
                            ctx.save_command(&DiffCommandRef::RemoveKey(key), true, true)?;
                            changed = true;
                        },
                    }
                }

                for (key, other_value) in other.iter() {
                    if !self.contains_key(key) {
                        ctx.save_command(&DiffCommandRef::AddKey(key), true, true)?;
                        ctx.save_command(&DiffCommandRef::Value(other_value), true, true)?;
                        changed = true;
                    }
                }

                if changed {
                    ctx.save_command::<()>(&DiffCommandRef::Exit, true, false)?;
                }
                Ok(changed)
            }

            fn apply<'de, A>(
                &mut self,
                seq: &mut A,
                ctx: &mut ApplyContext,
            ) -> Result<bool, <A as de::SeqAccess<'de>>::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut changed = false;
                while let Some(cmd) = ctx.read_next_command::<A, K>(seq)? {
                    use $crate::difference::DiffCommandValue::*;
                    use $crate::difference::DiffPathElementValue::*;
                    match cmd {
                        // we should not be getting fields when reading collection commands
                        Enter(Field(_)) => {
                            ctx.skip_value(seq)?;
                            break;
                        }
                        AddKey(key) => if let Some(Value(v)) = ctx.read_next_command(seq)? {
                            //changed |= self.insert(key, v).map(|old_val| v != old_val).unwrap_or(true);
                            self.insert(key, v);
                            changed = true;
                        } else {
                            panic!("Expected value after AddKey");
                        }
                        EnterKey(key) => if let Some(value_ref) = self.get_mut(&key) {
                            changed |= <V as SerdeDiff>::apply(value_ref, seq, ctx)?;
                        } else {
                            ctx.skip_value(seq)?;
                        }
                        RemoveKey(key) => changed |= self.remove(&key).is_some(),
                        _ => break,
                    }
                }
                Ok(changed)
            }
        }
    };
}

map_serde_diff!(HashMap<K, V>, Hash, Eq);
map_serde_diff!(BTreeMap<K, V>, Ord);

/// Implements SerdeDiff on a type given that it impls Serialize + Deserialize + PartialEq.
/// This makes the type a "terminal" type in the SerdeDiff hierarchy, meaning deeper inspection
/// will not be possible. Use the SerdeDiff derive macro for recursive field inspection.
#[macro_export]
macro_rules! opaque_serde_diff {
    ($t:ty) => {
        impl SerdeDiff for $t {
            fn diff<'a, S: $crate::_serde::ser::SerializeSeq>(
                &self,
                ctx: &mut $crate::DiffContext<'a, S>,
                other: &Self,
            ) -> Result<bool, S::Error> {
                if self != other {
                    ctx.save_value(other)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }

            fn apply<'de, A>(
                &mut self,
                seq: &mut A,
                ctx: &mut $crate::ApplyContext,
            ) -> Result<bool, <A as $crate::_serde::de::SeqAccess<'de>>::Error>
            where
                A: $crate::_serde::de::SeqAccess<'de>,
            {
                ctx.read_value(seq, self)
            }
        }
    };
}

// Implement `SerdeDiff` for primitive types and types defined in the standard library.
opaque_serde_diff!(bool);
opaque_serde_diff!(isize);
opaque_serde_diff!(i8);
opaque_serde_diff!(i16);
opaque_serde_diff!(i32);
opaque_serde_diff!(i64);
opaque_serde_diff!(usize);
opaque_serde_diff!(u8);
opaque_serde_diff!(u16);
opaque_serde_diff!(u32);
opaque_serde_diff!(u64);
opaque_serde_diff!(i128);
opaque_serde_diff!(u128);
opaque_serde_diff!(f32);
opaque_serde_diff!(f64);
opaque_serde_diff!(char);
opaque_serde_diff!(String);
opaque_serde_diff!(std::ffi::CString);
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
opaque_serde_diff!(std::ffi::OsString);
opaque_serde_diff!(std::num::NonZeroU8);
opaque_serde_diff!(std::num::NonZeroU16);
opaque_serde_diff!(std::num::NonZeroU32);
opaque_serde_diff!(std::num::NonZeroU64);
opaque_serde_diff!(std::time::Duration);
opaque_serde_diff!(std::time::SystemTime);
opaque_serde_diff!(std::net::IpAddr);
opaque_serde_diff!(std::net::Ipv4Addr);
opaque_serde_diff!(std::net::Ipv6Addr);
opaque_serde_diff!(std::net::SocketAddr);
opaque_serde_diff!(std::net::SocketAddrV4);
opaque_serde_diff!(std::net::SocketAddrV6);
opaque_serde_diff!(std::path::PathBuf);

impl<T: SerdeDiff + Serialize + for<'a> Deserialize<'a>> SerdeDiff for Option<T> {
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<bool, S::Error> {
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        let mut idx = 0;
        let mut need_exit = false;
        let mut changed = false;
        loop {
            let self_item = self_iter.next();
            let other_item = other_iter.next();
            match (self_item, other_item) {
                (None, None) => break,
                (Some(_), None) => {
                    let mut num_to_remove = 1;
                    while self_iter.next().is_some() {
                        num_to_remove += 1;
                    }
                    ctx.save_command::<()>(&DiffCommandRef::Remove(num_to_remove), true, true)?;
                    changed = true;
                }
                (None, Some(other_item)) => {
                    ctx.save_command::<()>(
                        &DiffCommandRef::Enter(DiffPathElementValue::AddToCollection),
                        false,
                        true,
                    )?;
                    ctx.save_command(&DiffCommandRef::Value(other_item), true, true)?;
                    need_exit = true;
                    changed = true;
                }
                (Some(self_item), Some(other_item)) => {
                    ctx.push_collection_index(idx);
                    if <T as SerdeDiff>::diff(self_item, ctx, other_item)? {
                        need_exit = true;
                        changed = true;
                    }
                    ctx.pop_path_element()?;
                }
            }
            idx += 1;
        }
        if need_exit {
            ctx.save_command::<()>(&DiffCommandRef::Exit, true, false)?;
        }
        Ok(changed)
    }

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<bool, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut changed = false;
        while let Some(cmd) = ctx.read_next_command::<A, T>(seq)? {
            use crate::difference::DiffCommandValue::*;
            use DiffPathElementValue::*;

            match cmd {
                // we should not be getting fields when reading collection commands
                Enter(Field(_)) => {
                    ctx.skip_value(seq)?;
                    break;
                }
                Enter(CollectionIndex(0)) => {
                    if let Some(value_ref) = self {
                        changed |= <T as SerdeDiff>::apply(value_ref, seq, ctx)?;
                    } else {
                        ctx.skip_value(seq)?;
                    }
                }
                Enter(AddToCollection) => {
                    if let Value(v) = ctx
                        .read_next_command(seq)?
                        .expect("Expected value after AddToCollection")
                    {
                        debug_assert!(self.is_none());
                        changed = true;
                        *self = Some(v);
                    } else {
                        panic!("Expected value after AddToCollection");
                    }
                }
                Remove(1) => {
                    *self = None;
                    changed = true;
                    break;
                }
                _ => break,
            }
        }
        Ok(changed)
    }
}

type Unit = ();
opaque_serde_diff!(Unit);
