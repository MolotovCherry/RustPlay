use cargo_player::{extract_use, TokenType};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

fn infer_benchmark(c: &mut Criterion) {
    let content = syn::parse_file(
        r#"
use foobar;
use proc_macro;

mod baz {
    use bar;

    struct Baz;
    impl Baz {
        fn boo() {
            match 0 {
                0 => {
                    use zero;
                }

                1 => {
                    let bar = || {
                        use bazz;
                        async {
                            if bar {
                                use ifbar;
                            } else {
                                use elsebar;
                            }
                        }
                    };
                }
            }
        }

        fn baz() {
            while true {
                for i in 0..4 {
                    use yes;
                }
            }
        }
    }
}


fn hoo() {
    use hi;
}
        "#,
    )
    .unwrap();

    c.bench_function("extract_use", |b| {
        b.iter_batched(
            || (content.items[0].clone(), vec![], vec![]),
            |(item, mut deps, mut mods)| {
                extract_use(TokenType::Item(black_box(item)), &mut deps, &mut mods)
            },
            BatchSize::LargeInput,
        );
    });
}

criterion_group!(benches, infer_benchmark);
criterion_main!(benches);
