#![allow(warnings)]
use async_gen::gen;

fn foo() {
    gen!({
        let f = "awd";
        yield {};
    });
    // gen(|mut t| async {
    //     async {
    //         t.yield_(42).await;
    //     };
    //     t.yield_({ 5 } + 5).await;
    //     t.return_(())
    // });
}
