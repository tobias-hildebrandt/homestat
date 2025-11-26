use reflect_instantiate::ReflectInstantiate;

#[test]
fn test() {
    #[allow(unused)]
    #[derive(ReflectInstantiate)]
    struct Outer {
        a: u32,
        b: u32,
        c: Inner,
    }

    #[derive(ReflectInstantiate)]
    struct Inner {
        deepest: u32,
    }

    let instance = Outer {
        a: 123u32,
        b: 456u32,
        c: Inner { deepest: 2u32 },
    };

    let expected = quote::quote! {
        Outer {
            a: 123u32,
            b: 456u32,
            c: Inner { deepest: 2u32, }, // trailing comma inside struct
        }
    };

    let actual = instance.instantiate();

    assert_eq!(expected.to_string(), actual.to_string());
}
