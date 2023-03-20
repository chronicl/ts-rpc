#![feature(type_name_of_val)]

pub use ts_rpc_core::*;
pub use ts_rpc_macros::ts_export;
// Todo: exporting self so TS derive macro works as long as we `use ts_rpc::ts_rs;`.
// Make this better, maybe move ts_rs fully into this crate.
pub use ts_rs::{self, TS};

#[test]
fn test_serde() {
    #[derive(serde::Serialize)]
    struct A1(u32);

    println!("one unnamed: {}", serde_json::to_string(&A1(1)).unwrap());

    #[derive(serde::Serialize)]
    struct A2(u32, u32);

    println!("two unnamed: {}", serde_json::to_string(&A2(1, 2)).unwrap());

    #[derive(TS)]
    struct A<T, U> {
        b: d::B<d::C<U>>,
        c: d::C<d::B<T>>,
    }

    mod d {
        use ts_rs::{self, TS};

        #[derive(TS, serde::Serialize, serde::Deserialize)]
        #[serde(rename = "F")]
        pub struct B<T> {
            d: T,
        }

        #[derive(TS)]
        pub struct C<T> {
            e: T,
        }
    }

    println!("one named: {}", A::<(), ()>::decl().unwrap());
}
