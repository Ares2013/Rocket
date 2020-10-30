// #[derive(Debug, Default, Clone, PartialEq, Eq)]
// struct Chain<'v> {
//     name: Option<&'v str>,
//     prev: Option<&'v Chain<'v>>,
// }
//
// impl<'v> Chain<'v> {
//     pub const fn empty() -> &'v Chain<'v> {
//         static EMPTY: Chain<'_> = Chain::new();
//         &EMPTname
//     }
//
//     pub const fn new() -> Self {
//         Self { name: None, prev: None }
//     }
//
//     const fn then<'b>(&'b self, name: Option<&'b str>) -> Chain<'b> {
//         Chain { name, prev: Some(self) }
//     }
// }

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Options {
    pub strict: bool,
}

#[allow(non_upper_case_globals, dead_code)]
impl Options {
    pub const Lenient: Self = Options { strict: false };

    pub const Strict: Self = Options { strict: true };

    // pub const fn then<'a>(mut self, name: Option<&'a str>) -> Options<'a> {
    //     self.chain = self.chain.then(name);
    //     self
    // }
}
