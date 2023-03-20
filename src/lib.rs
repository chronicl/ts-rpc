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

#[test]
fn test_min_specialization() {
    trait B {
        type S;

        fn b(&self) -> String;
    }

    // trait SpecializedSerialize: serde::Serialize + Sized {
    //     fn any<'a>(&'a self) -> &'a (dyn std::any::Any + 'a);
    // }

    // impl<T: std::any::Any + serde::Serialize + Sized> SpecializedSerialize for T {
    //     fn any<'a>(&'a self) -> &'a (dyn std::any::Any + 'a) {
    //         self
    //     }
    // }

    // let r = serde_json::to_string(&Result::<u32, u32>::Ok(1).boxed());
    // println!("{:?}", r);

    // trait Specialize {
    //     type Output;

    //     fn output(self) -> Self::Output;
    // }

    // impl<T> Specialize for T {
    //     default type Output = T;

    //     default fn output(self) -> Self::Output {
    //         self
    //     }
    // }

    // impl<T, E> Specialize for Result<T, E> {
    //     type Output = Result<T, E>;

    //     fn output(self) -> Self::Output {
    //         self
    //     }
    // }

    // struct SpecializedSerialize<T>(T);

    // impl<T: serde::Serialize> serde::Serialize for SpecializedSerialize<T> {
    //     default fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    //     where
    //         S: serde::Serializer,
    //     {
    //         self.0.serialize(serializer)
    //     }
    // }

    // impl<T: serde::Serialize, E> serde::Serialize for SpecializedSerialize<std::result::Result<T, E>> {
    //     fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    //     where
    //         S: serde::Serializer,
    //     {
    //         let my_result = match &self.0 {
    //             Ok(ok) => Result::Ok(ok),
    //             Err(err) => Result::Err(err),
    //         };
    //         my_result.serialize(serializer)
    //     }
    // }

    // struct F<T>(T);
    // impl<T: serde::Serialize> serde::Serialize for F<T> {
    //     default fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    //     where
    //         S: serde::Serializer,
    //     {
    //         self.0.serialize(serializer)
    //     }
    // }

    // impl<T: serde::Serialize> serde::Serialize for SpecializedSerialize<F<T>> {
    //     fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    //     where
    //         S: serde::Serializer,
    //     {
    //         self.0 .0.serialize(serializer)
    //     }
    // }

    // let k: Box<dyn serde::Serialize + Sized> = Box::new(0u32);
    // serde_json::to_string(&k);

    // impl<T> B for T {
    //     default fn b(&self) -> String {
    //         "default".to_string()
    //     }
    // }

    // impl B for A {
    //     type S = i32;

    //     fn b(&self) -> String {
    //         "A".to_string()
    //     }
    // }

    // assert_eq!(A {}.b(), "A");
    // assert_eq!(0u32.b(), "default");
}
