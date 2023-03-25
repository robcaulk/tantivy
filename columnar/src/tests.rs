use std::collections::HashMap;
use std::fmt::Debug;
use std::net::Ipv6Addr;

use common::DateTime;
use proptest::prelude::*;

use crate::column_values::MonotonicallyMappableToU128;
use crate::columnar::ColumnType;
use crate::dynamic_column::{DynamicColumn, DynamicColumnHandle};
use crate::value::{Coerce, NumericalValue};
use crate::{
    BytesColumn, Cardinality, Column, ColumnarReader, ColumnarWriter, RowId, StackMergeOrder,
};

#[test]
fn test_dataframe_writer_str() {
    let mut dataframe_writer = ColumnarWriter::default();
    dataframe_writer.record_str(1u32, "my_string", "hello");
    dataframe_writer.record_str(3u32, "my_string", "helloeee");
    let mut buffer: Vec<u8> = Vec::new();
    dataframe_writer.serialize(5, None, &mut buffer).unwrap();
    let columnar = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar.num_columns(), 1);
    let cols: Vec<DynamicColumnHandle> = columnar.read_columns("my_string").unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].num_bytes(), 89);
}

#[test]
fn test_dataframe_writer_bytes() {
    let mut dataframe_writer = ColumnarWriter::default();
    dataframe_writer.record_bytes(1u32, "my_string", b"hello");
    dataframe_writer.record_bytes(3u32, "my_string", b"helloeee");
    let mut buffer: Vec<u8> = Vec::new();
    dataframe_writer.serialize(5, None, &mut buffer).unwrap();
    let columnar = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar.num_columns(), 1);
    let cols: Vec<DynamicColumnHandle> = columnar.read_columns("my_string").unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].num_bytes(), 89);
}

#[test]
fn test_dataframe_writer_bool() {
    let mut dataframe_writer = ColumnarWriter::default();
    dataframe_writer.record_bool(1u32, "bool.value", false);
    dataframe_writer.record_bool(3u32, "bool.value", true);
    let mut buffer: Vec<u8> = Vec::new();
    dataframe_writer.serialize(5, None, &mut buffer).unwrap();
    let columnar = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar.num_columns(), 1);
    let cols: Vec<DynamicColumnHandle> = columnar.read_columns("bool.value").unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].num_bytes(), 22);
    assert_eq!(cols[0].column_type(), ColumnType::Bool);
    let dyn_bool_col = cols[0].open().unwrap();
    let DynamicColumn::Bool(bool_col) = dyn_bool_col else { panic!(); };
    let vals: Vec<Option<bool>> = (0..5).map(|row_id| bool_col.first(row_id)).collect();
    assert_eq!(&vals, &[None, Some(false), None, Some(true), None,]);
}

#[test]
fn test_dataframe_writer_u64_multivalued() {
    let mut dataframe_writer = ColumnarWriter::default();
    dataframe_writer.record_numerical(2u32, "divisor", 2u64);
    dataframe_writer.record_numerical(3u32, "divisor", 3u64);
    dataframe_writer.record_numerical(4u32, "divisor", 2u64);
    dataframe_writer.record_numerical(5u32, "divisor", 5u64);
    dataframe_writer.record_numerical(6u32, "divisor", 2u64);
    dataframe_writer.record_numerical(6u32, "divisor", 3u64);
    let mut buffer: Vec<u8> = Vec::new();
    dataframe_writer.serialize(7, None, &mut buffer).unwrap();
    let columnar = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar.num_columns(), 1);
    let cols: Vec<DynamicColumnHandle> = columnar.read_columns("divisor").unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].num_bytes(), 29);
    let dyn_i64_col = cols[0].open().unwrap();
    let DynamicColumn::I64(divisor_col) = dyn_i64_col else { panic!(); };
    assert_eq!(
        divisor_col.get_cardinality(),
        crate::Cardinality::Multivalued
    );
    assert_eq!(divisor_col.num_docs(), 7);
}

#[test]
fn test_dataframe_writer_ip_addr() {
    let mut dataframe_writer = ColumnarWriter::default();
    dataframe_writer.record_ip_addr(1, "ip_addr", Ipv6Addr::from_u128(1001));
    dataframe_writer.record_ip_addr(3, "ip_addr", Ipv6Addr::from_u128(1050));
    let mut buffer: Vec<u8> = Vec::new();
    dataframe_writer.serialize(5, None, &mut buffer).unwrap();
    let columnar = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar.num_columns(), 1);
    let cols: Vec<DynamicColumnHandle> = columnar.read_columns("ip_addr").unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].num_bytes(), 42);
    assert_eq!(cols[0].column_type(), ColumnType::IpAddr);
    let dyn_bool_col = cols[0].open().unwrap();
    let DynamicColumn::IpAddr(ip_col) = dyn_bool_col else { panic!(); };
    let vals: Vec<Option<Ipv6Addr>> = (0..5).map(|row_id| ip_col.first(row_id)).collect();
    assert_eq!(
        &vals,
        &[
            None,
            Some(Ipv6Addr::from_u128(1001)),
            None,
            Some(Ipv6Addr::from_u128(1050)),
            None,
        ]
    );
}

#[test]
fn test_dataframe_writer_numerical() {
    let mut dataframe_writer = ColumnarWriter::default();
    dataframe_writer.record_numerical(1u32, "srical.value", NumericalValue::U64(12u64));
    dataframe_writer.record_numerical(2u32, "srical.value", NumericalValue::U64(13u64));
    dataframe_writer.record_numerical(4u32, "srical.value", NumericalValue::U64(15u64));
    let mut buffer: Vec<u8> = Vec::new();
    dataframe_writer.serialize(6, None, &mut buffer).unwrap();
    let columnar = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar.num_columns(), 1);
    let cols: Vec<DynamicColumnHandle> = columnar.read_columns("srical.value").unwrap();
    assert_eq!(cols.len(), 1);
    // Right now this 31 bytes are spent as follows
    //
    // - header 14 bytes
    // - vals  8 //< due to padding? could have been 1byte?.
    // - null footer 6 bytes
    assert_eq!(cols[0].num_bytes(), 33);
    let column = cols[0].open().unwrap();
    let DynamicColumn::I64(column_i64) = column else { panic!(); };
    assert_eq!(column_i64.index.get_cardinality(), Cardinality::Optional);
    assert_eq!(column_i64.first(0), None);
    assert_eq!(column_i64.first(1), Some(12i64));
    assert_eq!(column_i64.first(2), Some(13i64));
    assert_eq!(column_i64.first(3), None);
    assert_eq!(column_i64.first(4), Some(15i64));
    assert_eq!(column_i64.first(5), None);
    assert_eq!(column_i64.first(6), None); //< we can change the spec for that one.
}

#[test]
fn test_dictionary_encoded_str() {
    let mut buffer = Vec::new();
    let mut columnar_writer = ColumnarWriter::default();
    columnar_writer.record_str(1, "my.column", "a");
    columnar_writer.record_str(3, "my.column", "c");
    columnar_writer.record_str(3, "my.column2", "different_column!");
    columnar_writer.record_str(4, "my.column", "b");
    columnar_writer.serialize(5, None, &mut buffer).unwrap();
    let columnar_reader = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar_reader.num_columns(), 2);
    let col_handles = columnar_reader.read_columns("my.column").unwrap();
    assert_eq!(col_handles.len(), 1);
    let DynamicColumn::Str(str_col) = col_handles[0].open().unwrap() else  { panic!(); };
    let index: Vec<Option<u64>> = (0..5).map(|row_id| str_col.ords().first(row_id)).collect();
    assert_eq!(index, &[None, Some(0), None, Some(2), Some(1)]);
    assert_eq!(str_col.num_rows(), 5);
    let mut term_buffer = String::new();
    let term_ords = str_col.ords();
    assert_eq!(term_ords.first(0), None);
    assert_eq!(term_ords.first(1), Some(0));
    str_col.ord_to_str(0u64, &mut term_buffer).unwrap();
    assert_eq!(term_buffer, "a");
    assert_eq!(term_ords.first(2), None);
    assert_eq!(term_ords.first(3), Some(2));
    str_col.ord_to_str(2u64, &mut term_buffer).unwrap();
    assert_eq!(term_buffer, "c");
    assert_eq!(term_ords.first(4), Some(1));
    str_col.ord_to_str(1u64, &mut term_buffer).unwrap();
    assert_eq!(term_buffer, "b");
}

#[test]
fn test_dictionary_encoded_bytes() {
    let mut buffer = Vec::new();
    let mut columnar_writer = ColumnarWriter::default();
    columnar_writer.record_bytes(1, "my.column", b"a");
    columnar_writer.record_bytes(3, "my.column", b"c");
    columnar_writer.record_bytes(3, "my.column2", b"different_column!");
    columnar_writer.record_bytes(4, "my.column", b"b");
    columnar_writer.serialize(5, None, &mut buffer).unwrap();
    let columnar_reader = ColumnarReader::open(buffer).unwrap();
    assert_eq!(columnar_reader.num_columns(), 2);
    let col_handles = columnar_reader.read_columns("my.column").unwrap();
    assert_eq!(col_handles.len(), 1);
    let DynamicColumn::Bytes(bytes_col) = col_handles[0].open().unwrap() else  { panic!(); };
    let index: Vec<Option<u64>> = (0..5)
        .map(|row_id| bytes_col.ords().first(row_id))
        .collect();
    assert_eq!(index, &[None, Some(0), None, Some(2), Some(1)]);
    assert_eq!(bytes_col.num_rows(), 5);
    let mut term_buffer = Vec::new();
    let term_ords = bytes_col.ords();
    assert_eq!(term_ords.first(0), None);
    assert_eq!(term_ords.first(1), Some(0));
    bytes_col
        .dictionary
        .ord_to_term(0u64, &mut term_buffer)
        .unwrap();
    assert_eq!(term_buffer, b"a");
    assert_eq!(term_ords.first(2), None);
    assert_eq!(term_ords.first(3), Some(2));
    bytes_col
        .dictionary
        .ord_to_term(2u64, &mut term_buffer)
        .unwrap();
    assert_eq!(term_buffer, b"c");
    assert_eq!(term_ords.first(4), Some(1));
    bytes_col
        .dictionary
        .ord_to_term(1u64, &mut term_buffer)
        .unwrap();
    assert_eq!(term_buffer, b"b");
}

fn num_strategy() -> impl Strategy<Value = NumericalValue> {
    prop_oneof![
        Just(NumericalValue::U64(0u64)),
        Just(NumericalValue::U64(u64::MAX)),
        Just(NumericalValue::I64(0i64)),
        Just(NumericalValue::I64(i64::MIN)),
        Just(NumericalValue::I64(i64::MAX)),
        Just(NumericalValue::F64(1.2f64)),
    ]
}

#[derive(Debug, Clone, Copy)]
enum ColumnValue {
    Str(&'static str),
    Bytes(&'static [u8]),
    Numerical(NumericalValue),
    IpAddr(Ipv6Addr),
    Bool(bool),
    DateTime(DateTime),
}

impl ColumnValue {
    pub(crate) fn column_type_category(&self) -> ColumnTypeCategory {
        match self {
            ColumnValue::Str(_) => ColumnTypeCategory::Str,
            ColumnValue::Bytes(_) => ColumnTypeCategory::Bytes,
            ColumnValue::Numerical(numerical_val) => ColumnTypeCategory::Numerical,
            ColumnValue::IpAddr(_) => ColumnTypeCategory::IpAddr,
            ColumnValue::Bool(_) => ColumnTypeCategory::Bool,
            ColumnValue::DateTime(_) => ColumnTypeCategory::DateTime,
        }
    }
}

fn column_name_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("c1"), Just("c2")]
}

fn string_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("a"), Just("b")]
}

fn bytes_strategy() -> impl Strategy<Value = &'static [u8]> {
    prop_oneof![Just(&[0u8][..]), Just(&[1u8][..])]
}

// A random column value
fn column_value_strategy() -> impl Strategy<Value = ColumnValue> {
    prop_oneof![
        string_strategy().prop_map(|s| ColumnValue::Str(s)),
        bytes_strategy().prop_map(|b| ColumnValue::Bytes(b)),
        num_strategy().prop_map(|n| ColumnValue::Numerical(n)),
        (1u16..3u16).prop_map(|ip_addr_byte| ColumnValue::IpAddr(Ipv6Addr::new(
            127,
            0,
            0,
            0,
            0,
            0,
            0,
            ip_addr_byte
        ))),
        any::<bool>().prop_map(|b| ColumnValue::Bool(b)),
        (0_679_723_993i64..1_679_723_995i64)
            .prop_map(|val| { ColumnValue::DateTime(DateTime::from_timestamp_secs(val)) })
    ]
}

// A document contains up to 4 values.
fn doc_strategy() -> impl Strategy<Value = Vec<(&'static str, ColumnValue)>> {
    proptest::collection::vec((column_name_strategy(), column_value_strategy()), 0..4)
}

// A columnar contains up to 2 docs.
fn columnar_docs_strategy() -> impl Strategy<Value = Vec<Vec<(&'static str, ColumnValue)>>> {
    proptest::collection::vec(doc_strategy(), 0..=2)
}

fn columnar_docs_and_mapping_strategy(
) -> impl Strategy<Value = (Vec<Vec<(&'static str, ColumnValue)>>, Vec<RowId>)> {
    columnar_docs_strategy().prop_flat_map(|docs| {
        permutation_strategy(docs.len()).prop_map(move |permutation| (docs.clone(), permutation))
    })
}

fn permutation_strategy(n: usize) -> impl Strategy<Value = Vec<RowId>> {
    Just((0u32..n as RowId).collect()).prop_shuffle()
}

fn build_columnar_with_mapping(
    docs: &[Vec<(&'static str, ColumnValue)>],
    old_to_new_row_ids_opt: Option<&[RowId]>,
) -> ColumnarReader {
    let num_docs = docs.len() as u32;
    let mut buffer = Vec::new();
    let mut columnar_writer = ColumnarWriter::default();
    for (doc_id, vals) in docs.iter().enumerate() {
        for (column_name, col_val) in vals {
            match *col_val {
                ColumnValue::Str(str_val) => {
                    columnar_writer.record_str(doc_id as u32, column_name, str_val);
                }
                ColumnValue::Bytes(bytes) => {
                    columnar_writer.record_bytes(doc_id as u32, column_name, bytes)
                }
                ColumnValue::Numerical(num) => {
                    columnar_writer.record_numerical(doc_id as u32, column_name, num);
                }
                ColumnValue::IpAddr(ip_addr) => {
                    columnar_writer.record_ip_addr(doc_id as u32, column_name, ip_addr);
                }
                ColumnValue::Bool(bool_val) => {
                    columnar_writer.record_bool(doc_id as u32, column_name, bool_val);
                }
                ColumnValue::DateTime(date_time) => {
                    columnar_writer.record_datetime(doc_id as u32, column_name, date_time);
                }
            }
        }
    }
    columnar_writer
        .serialize(num_docs, old_to_new_row_ids_opt, &mut buffer)
        .unwrap();
    let columnar_reader = ColumnarReader::open(buffer).unwrap();
    columnar_reader
}

fn build_columnar(docs: &[Vec<(&'static str, ColumnValue)>]) -> ColumnarReader {
    build_columnar_with_mapping(docs, None)
}

fn assert_columnar_eq(left: &ColumnarReader, right: &ColumnarReader) {
    assert_eq!(left.num_rows(), right.num_rows());
    let left_columns = left.list_columns().unwrap();
    let right_columns = right.list_columns().unwrap();
    assert_eq!(left_columns.len(), right_columns.len());
    for i in 0..left_columns.len() {
        assert_eq!(left_columns[i].0, right_columns[i].0);
        let left_column = left_columns[i].1.open().unwrap();
        let right_column = right_columns[i].1.open().unwrap();
        assert_dyn_column_eq(&left_column, &right_column);
    }
}

fn assert_column_eq<T: PartialEq + Copy>(left: &Column<T>, right: &Column<T>) {}

fn assert_bytes_column_eq(left: &BytesColumn, right: &BytesColumn) {}

fn assert_dyn_column_eq(left_dyn_column: &DynamicColumn, right_dyn_column: &DynamicColumn) {
    assert_eq!(
        &left_dyn_column.column_type(),
        &right_dyn_column.column_type()
    );
    assert_eq!(
        &left_dyn_column.get_cardinality(),
        &right_dyn_column.get_cardinality()
    );
    match &(left_dyn_column, right_dyn_column) {
        (DynamicColumn::Bool(left_col), DynamicColumn::Bool(right_col)) => {
            assert_column_eq(left_col, right_col);
        }
        (DynamicColumn::I64(left_col), DynamicColumn::I64(right_col)) => {
            assert_column_eq(left_col, right_col);
        }
        (DynamicColumn::U64(left_col), DynamicColumn::U64(right_col)) => {
            assert_column_eq(left_col, right_col);
        }
        (DynamicColumn::F64(left_col), DynamicColumn::F64(right_col)) => {
            assert_column_eq(left_col, right_col);
        }
        (DynamicColumn::DateTime(left_col), DynamicColumn::DateTime(right_col)) => {
            assert_column_eq(left_col, right_col);
        }
        (DynamicColumn::IpAddr(left_col), DynamicColumn::IpAddr(right_col)) => {
            assert_column_eq(left_col, right_col);
        }
        (DynamicColumn::Bytes(left_col), DynamicColumn::Bytes(right_col)) => {
            assert_bytes_column_eq(left_col, right_col);
        }
        (DynamicColumn::Str(left_col), DynamicColumn::Str(right_col)) => {
            assert_bytes_column_eq(left_col, right_col);
        }
        _ => {
            unreachable!()
        }
    }
}

trait AssertEqualToColumnValue {
    fn assert_equal_to_column_value(&self, column_value: &ColumnValue);
}

use crate::columnar::ColumnTypeCategory;

impl AssertEqualToColumnValue for bool {
    fn assert_equal_to_column_value(&self, column_value: &ColumnValue) {
        let ColumnValue::Bool(val) = column_value else { panic!() };
        assert_eq!(self, val);
    }
}

impl AssertEqualToColumnValue for Ipv6Addr {
    fn assert_equal_to_column_value(&self, column_value: &ColumnValue) {
        let ColumnValue::IpAddr(val) = column_value else { panic!() };
        assert_eq!(self, val);
    }
}

impl<T: Coerce + PartialEq + Debug + Into<NumericalValue>> AssertEqualToColumnValue for T {
    fn assert_equal_to_column_value(&self, column_value: &ColumnValue) {
        let ColumnValue::Numerical(num) = column_value else { panic!() };
        assert_eq!(self, &T::coerce(*num));
    }
}

impl AssertEqualToColumnValue for DateTime {
    fn assert_equal_to_column_value(&self, column_value: &ColumnValue) {
        let ColumnValue::DateTime(dt) = column_value else { panic!() };
        assert_eq!(self, dt);
    }
}

fn assert_column_values<
    T: AssertEqualToColumnValue + PartialEq + Copy + PartialOrd + Debug + Send + Sync + 'static,
>(
    col: &Column<T>,
    expected: &HashMap<u32, Vec<&ColumnValue>>,
) {
    let mut num_non_empty_rows = 0;
    for doc in 0..col.num_docs() {
        let doc_vals: Vec<T> = col.values_for_doc(doc).collect();
        if doc_vals.is_empty() {
            continue;
        }
        num_non_empty_rows += 1;
        let expected_vals = expected.get(&doc).unwrap();
        assert_eq!(doc_vals.len(), expected_vals.len());
        for (val, &expected) in doc_vals.iter().zip(expected_vals.iter()) {
            val.assert_equal_to_column_value(expected)
        }
    }
    assert_eq!(num_non_empty_rows, expected.len());
}

fn assert_bytes_column_values(
    col: &BytesColumn,
    expected: &HashMap<u32, Vec<&ColumnValue>>,
    is_str: bool,
) {
    let mut num_non_empty_rows = 0;
    let mut buffer = Vec::new();
    for doc in 0..col.term_ord_column.num_docs() {
        let doc_vals: Vec<u64> = col.term_ords(doc).collect();
        if doc_vals.is_empty() {
            continue;
        }
        let expected_vals = expected.get(&doc).unwrap();
        assert_eq!(doc_vals.len(), expected_vals.len());
        for (&expected_col_val, &ord) in expected_vals.iter().zip(&doc_vals) {
            col.ord_to_bytes(ord, &mut buffer).unwrap();
            match expected_col_val {
                ColumnValue::Str(str_val) => {
                    assert!(is_str);
                    assert_eq!(str_val.as_bytes(), &buffer);
                }
                ColumnValue::Bytes(bytes_val) => {
                    assert!(!is_str);
                    assert_eq!(bytes_val, &buffer);
                }
                _ => {
                    panic!();
                }
            }
        }
        num_non_empty_rows += 1;
    }
    assert_eq!(num_non_empty_rows, expected.len());
}

proptest! {
    /// This proptest attempts to create a tiny columnar based of up to 3 rows, and checks that the resulting
    /// columnar matches the row data.
    #[test]
    fn test_single_columnar_builder_proptest(docs in columnar_docs_strategy()) {
        let columnar = build_columnar(&docs[..]);
        assert_eq!(columnar.num_rows() as usize, docs.len());
        let mut expected_columns: HashMap<(&str, ColumnTypeCategory), HashMap<u32, Vec<&ColumnValue>> > = Default::default();
        for (doc_id, doc_vals) in docs.iter().enumerate() {
            for (col_name, col_val) in doc_vals {
                expected_columns
                    .entry((col_name, col_val.column_type_category()))
                    .or_default()
                    .entry(doc_id as u32)
                    .or_default()
                    .push(col_val);
            }
        }
        let column_list = columnar.list_columns().unwrap();
        assert_eq!(expected_columns.len(), column_list.len());
        for (column_name, column) in column_list {
            let dynamic_column = column.open().unwrap();
            let col_category: ColumnTypeCategory = dynamic_column.column_type().into();
            let expected_col_values: &HashMap<u32, Vec<&ColumnValue>> = expected_columns.get(&(column_name.as_str(), col_category)).unwrap();
            match &dynamic_column {
                DynamicColumn::Bool(col) =>
                    assert_column_values(col, expected_col_values),
                DynamicColumn::I64(col) =>
                    assert_column_values(col, expected_col_values),
                DynamicColumn::U64(col) =>
                    assert_column_values(col, expected_col_values),
                DynamicColumn::F64(col) =>
                    assert_column_values(col, expected_col_values),
                DynamicColumn::IpAddr(col) =>
                    assert_column_values(col, expected_col_values),
                DynamicColumn::DateTime(col) =>
                    assert_column_values(col, expected_col_values),
                DynamicColumn::Bytes(col) =>
                    assert_bytes_column_values(col, expected_col_values, false),
                DynamicColumn::Str(col) =>
                    assert_bytes_column_values(col, expected_col_values, true),
            }
        }
    }

    /// Same as `test_single_columnar_builder_proptest` but with a shuffling mapping.
    #[test]
    fn test_single_columnar_builder_with_shuffle_proptest((docs, mapping) in columnar_docs_and_mapping_strategy()) {
        let columnar = build_columnar_with_mapping(&docs[..], Some(&mapping));
        assert_eq!(columnar.num_rows() as usize, docs.len());
        let mut expected_columns: HashMap<(&str, ColumnTypeCategory), HashMap<u32, Vec<&ColumnValue>> > = Default::default();
        for (doc_id, doc_vals) in docs.iter().enumerate() {
            for (col_name, col_val) in doc_vals {
                expected_columns
                    .entry((col_name, col_val.column_type_category()))
                    .or_default()
                    .entry(mapping[doc_id])
                    .or_default()
                    .push(col_val);
            }
        }
        let column_list = columnar.list_columns().unwrap();
        assert_eq!(expected_columns.len(), column_list.len());
        for (column_name, column) in column_list {
            let dynamic_column = column.open().unwrap();
            let col_category: ColumnTypeCategory = dynamic_column.column_type().into();
            let expected_col_values: &HashMap<u32, Vec<&ColumnValue>> = expected_columns.get(&(column_name.as_str(), col_category)).unwrap();
            for doc_id in 0..columnar.num_rows() {
                match &dynamic_column {
                    DynamicColumn::Bool(col) =>
                        assert_column_values(col, expected_col_values),
                    DynamicColumn::I64(col) =>
                        assert_column_values(col, expected_col_values),
                    DynamicColumn::U64(col) =>
                        assert_column_values(col, expected_col_values),
                    DynamicColumn::F64(col) =>
                        assert_column_values(col, expected_col_values),
                    DynamicColumn::IpAddr(col) =>
                        assert_column_values(col, expected_col_values),
                    DynamicColumn::DateTime(col) =>
                        assert_column_values(col, expected_col_values),
                    DynamicColumn::Bytes(col) =>
                        assert_bytes_column_values(col, expected_col_values, false),
                    DynamicColumn::Str(col) =>
                        assert_bytes_column_values(col, expected_col_values, true),
                }
            }
        }
    }

    /// This tests create 2 or 3 random small columnar and attempts to merge them.
    /// It compares the resulting merged dataframe with what would have been obtained by building the
    /// dataframe from the concatenated rows to begin with.
    #[test]
    fn test_columnar_merge_proptest(columnar_docs in proptest::collection::vec(columnar_docs_strategy(), 2..=3)) {
        let columnar_readers: Vec<ColumnarReader> = columnar_docs.iter()
            .map(|docs| build_columnar(&docs[..]))
            .collect::<Vec<_>>();
        let columnar_readers_arr: Vec<&ColumnarReader> = columnar_readers.iter().collect();
        let mut output: Vec<u8> = Vec::new();
        let stack_merge_order = StackMergeOrder::stack(&columnar_readers_arr[..]);
        crate::merge_columnar(&columnar_readers_arr[..], &[], crate::MergeRowOrder::Stack(stack_merge_order), &mut output).unwrap();
        let merged_columnar = ColumnarReader::open(output).unwrap();
        let concat_rows: Vec<Vec<(&'static str, ColumnValue)>> = columnar_docs.iter().cloned().flatten().collect();
        let expected_merged_columnar = build_columnar(&concat_rows[..]);
        assert_columnar_eq(&merged_columnar, &expected_merged_columnar);
    }


}

#[test]
fn test_columnar_failing_test() {
    let columnar_docs: Vec<Vec<Vec<(&str, ColumnValue)>>> =
        vec![vec![], vec![vec![("c1", ColumnValue::Str("a"))]]];
    let columnar_readers: Vec<ColumnarReader> = columnar_docs
        .iter()
        .map(|docs| build_columnar(&docs[..]))
        .collect::<Vec<_>>();
    let columnar_readers_arr: Vec<&ColumnarReader> = columnar_readers.iter().collect();
    let mut output: Vec<u8> = Vec::new();
    let stack_merge_order = StackMergeOrder::stack(&columnar_readers_arr[..]);
    crate::merge_columnar(
        &columnar_readers_arr[..],
        &[],
        crate::MergeRowOrder::Stack(stack_merge_order),
        &mut output,
    )
    .unwrap();
    let merged_columnar = ColumnarReader::open(output).unwrap();
    let concat_rows: Vec<Vec<(&'static str, ColumnValue)>> =
        columnar_docs.iter().cloned().flatten().collect();
    let expected_merged_columnar = build_columnar(&concat_rows[..]);
    assert_columnar_eq(&merged_columnar, &expected_merged_columnar);
}

// TODO add non trivial remap and merge
// TODO test required
// TODO add support for empty columnar.
