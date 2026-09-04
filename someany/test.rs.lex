
 INFO rustc_parse::lexer JUHYUNG lex_token_trees: returning Ok with stream: TokenStream(
     [
         Token(
             Token {
                 kind: Ident(
                     "fn",
                     No,
                 ),
                 span: ./someany/test.rs:3:1: 3:3 (#0),
             },
             Alone,
         ),
         Token(
             Token {
                 kind: Ident(
                     "foo",
                     No,
                 ),
                 span: ./someany/test.rs:3:4: 3:7 (#0),
             },
             JointHidden,
         ),
         Delimited(
             DelimSpan {
                 open: ./someany/test.rs:3:7: 3:8 (#0),
                 close: ./someany/test.rs:3:14: 3:15 (#0),
             },
             DelimSpacing {
                 open: JointHidden,
                 close: Alone,
             },
             Parenthesis,
             TokenStream(
                 [
                     Token(
                         Token {
                             kind: Ident(
                                 "x",
                                 No,
                             ),
                             span: ./someany/test.rs:3:8: 3:9 (#0),
                         },
                         Joint,
                     ),
                     Token(
                         Token {
                             kind: Colon,
                             span: ./someany/test.rs:3:9: 3:10 (#0),
                         },
                         Alone,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "i32",
                                 No,
                             ),
                             span: ./someany/test.rs:3:11: 3:14 (#0),
                         },
                         JointHidden,
                     ),
                 ],
             ),
         ),
         Token(
             Token {
                 kind: RArrow,
                 span: ./someany/test.rs:3:16: 3:18 (#0),
             },
             Alone,
         ),
         Token(
             Token {
                 kind: Ident(
                     "some",
                     No,
                 ),
                 span: ./someany/test.rs:3:19: 3:23 (#0),
             },
             Alone,
         ),
         Token(
             Token {
                 kind: Ident(
                     "Iterator",
                     No,
                 ),
                 span: ./someany/test.rs:3:24: 3:32 (#0),
             },
             Joint,
         ),
         Token(
             Token {
                 kind: Lt,
                 span: ./someany/test.rs:3:32: 3:33 (#0),
             },
             JointHidden,
         ),
         Token(
             Token {
                 kind: Ident(
                     "Item",
                     No,
                 ),
                 span: ./someany/test.rs:3:33: 3:37 (#0),
             },
             Alone,
         ),
         Token(
             Token {
                 kind: Eq,
                 span: ./someany/test.rs:3:38: 3:39 (#0),
             },
             Alone,
         ),
         Token(
             Token {
                 kind: Ident(
                     "i32",
                     No,
                 ),
                 span: ./someany/test.rs:3:40: 3:43 (#0),
             },
             Joint,
         ),
         Token(
             Token {
                 kind: Gt,
                 span: ./someany/test.rs:3:43: 3:44 (#0),
             },
             Alone,
         ),
         Delimited(
             DelimSpan {
                 open: ./someany/test.rs:3:45: 3:46 (#0),
                 close: ./someany/test.rs:5:1: 5:2 (#0),
             },
             DelimSpacing {
                 open: Alone,
                 close: Alone,
             },
             Brace,
             TokenStream(
                 [
                     Delimited(
                         DelimSpan {
                             open: ./someany/test.rs:4:5: 4:6 (#0),
                             close: ./someany/test.rs:4:10: 4:11 (#0),
                         },
                         DelimSpacing {
                             open: JointHidden,
                             close: Joint,
                         },
                         Parenthesis,
                         TokenStream(
                             [
                                 Token(
                                     Token {
                                         kind: Literal(
                                             Lit {
                                                 kind: Integer,
                                                 symbol: "0",
                                                 suffix: None,
                                             },
                                         ),
                                         span: ./someany/test.rs:4:6: 4:7 (#0),
                                     },
                                     Joint,
                                 ),
                                 Token(
                                     Token {
                                         kind: DotDot,
                                         span: ./someany/test.rs:4:7: 4:9 (#0),
                                     },
                                     JointHidden,
                                 ),
                                 Token(
                                     Token {
                                         kind: Ident(
                                             "x",
                                             No,
                                         ),
                                         span: ./someany/test.rs:4:9: 4:10 (#0),
                                     },
                                     JointHidden,
                                 ),
                             ],
                         ),
                     ),
                     Token(
                         Token {
                             kind: Dot,
                             span: ./someany/test.rs:4:11: 4:12 (#0),
                         },
                         JointHidden,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "map",
                                 No,
                             ),
                             span: ./someany/test.rs:4:12: 4:15 (#0),
                         },
                         JointHidden,
                     ),
                     Delimited(
                         DelimSpan {
                             open: ./someany/test.rs:4:15: 4:16 (#0),
                             close: ./someany/test.rs:4:25: 4:26 (#0),
                         },
                         DelimSpacing {
                             open: Joint,
                             close: Alone,
                         },
                         Parenthesis,
                         TokenStream(
                             [
                                 Token(
                                     Token {
                                         kind: Or,
                                         span: ./someany/test.rs:4:16: 4:17 (#0),
                                     },
                                     JointHidden,
                                 ),
                                 Token(
                                     Token {
                                         kind: Ident(
                                             "i",
                                             No,
                                         ),
                                         span: ./someany/test.rs:4:17: 4:18 (#0),
                                     },
                                     Joint,
                                 ),
                                 Token(
                                     Token {
                                         kind: Or,
                                         span: ./someany/test.rs:4:18: 4:19 (#0),
                                     },
                                     Alone,
                                 ),
                                 Token(
                                     Token {
                                         kind: Ident(
                                             "i",
                                             No,
                                         ),
                                         span: ./someany/test.rs:4:20: 4:21 (#0),
                                     },
                                     Alone,
                                 ),
                                 Token(
                                     Token {
                                         kind: Star,
                                         span: ./someany/test.rs:4:22: 4:23 (#0),
                                     },
                                     Alone,
                                 ),
                                 Token(
                                     Token {
                                         kind: Literal(
                                             Lit {
                                                 kind: Integer,
                                                 symbol: "2",
                                                 suffix: None,
                                             },
                                         ),
                                         span: ./someany/test.rs:4:24: 4:25 (#0),
                                     },
                                     JointHidden,
                                 ),
                             ],
                         ),
                     ),
                 ],
             ),
         ),
         Token(
             Token {
                 kind: Ident(
                     "fn",
                     No,
                 ),
                 span: ./someany/test.rs:7:1: 7:3 (#0),
             },
             Alone,
         ),
         Token(
             Token {
                 kind: Ident(
                     "main",
                     No,
                 ),
                 span: ./someany/test.rs:7:4: 7:8 (#0),
             },
             JointHidden,
         ),
         Delimited(
             DelimSpan {
                 open: ./someany/test.rs:7:8: 7:9 (#0),
                 close: ./someany/test.rs:7:9: 7:10 (#0),
             },
             DelimSpacing {
                 open: JointHidden,
                 close: Alone,
             },
             Parenthesis,
             TokenStream(
                 [],
             ),
         ),
         Delimited(
             DelimSpan {
                 open: ./someany/test.rs:7:11: 7:12 (#0),
                 close: ./someany/test.rs:10:1: 10:2 (#0),
             },
             DelimSpacing {
                 open: Alone,
                 close: Alone,
             },
             Brace,
             TokenStream(
                 [
                     Token(
                         Token {
                             kind: Ident(
                                 "let",
                                 No,
                             ),
                             span: ./someany/test.rs:8:5: 8:8 (#0),
                         },
                         Alone,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "v",
                                 No,
                             ),
                             span: ./someany/test.rs:8:9: 8:10 (#0),
                         },
                         Joint,
                     ),
                     Token(
                         Token {
                             kind: Colon,
                             span: ./someany/test.rs:8:10: 8:11 (#0),
                         },
                         Alone,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "Vec",
                                 No,
                             ),
                             span: ./someany/test.rs:8:12: 8:15 (#0),
                         },
                         Joint,
                     ),
                     Token(
                         Token {
                             kind: Lt,
                             span: ./someany/test.rs:8:15: 8:16 (#0),
                         },
                         JointHidden,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "i32",
                                 No,
                             ),
                             span: ./someany/test.rs:8:16: 8:19 (#0),
                         },
                         Joint,
                     ),
                     Token(
                         Token {
                             kind: Gt,
                             span: ./someany/test.rs:8:19: 8:20 (#0),
                         },
                         Alone,
                     ),
                     Token(
                         Token {
                             kind: Eq,
                             span: ./someany/test.rs:8:21: 8:22 (#0),
                         },
                         Alone,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "foo",
                                 No,
                             ),
                             span: ./someany/test.rs:8:23: 8:26 (#0),
                         },
                         JointHidden,
                     ),
                     Delimited(
                         DelimSpan {
                             open: ./someany/test.rs:8:26: 8:27 (#0),
                             close: ./someany/test.rs:8:28: 8:29 (#0),
                         },
                         DelimSpacing {
                             open: JointHidden,
                             close: Joint,
                         },
                         Parenthesis,
                         TokenStream(
                             [
                                 Token(
                                     Token {
                                         kind: Literal(
                                             Lit {
                                                 kind: Integer,
                                                 symbol: "5",
                                                 suffix: None,
                                             },
                                         ),
                                         span: ./someany/test.rs:8:27: 8:28 (#0),
                                     },
                                     JointHidden,
                                 ),
                             ],
                         ),
                     ),
                     Token(
                         Token {
                             kind: Dot,
                             span: ./someany/test.rs:8:29: 8:30 (#0),
                         },
                         JointHidden,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "collect",
                                 No,
                             ),
                             span: ./someany/test.rs:8:30: 8:37 (#0),
                         },
                         JointHidden,
                     ),
                     Delimited(
                         DelimSpan {
                             open: ./someany/test.rs:8:37: 8:38 (#0),
                             close: ./someany/test.rs:8:38: 8:39 (#0),
                         },
                         DelimSpacing {
                             open: JointHidden,
                             close: Joint,
                         },
                         Parenthesis,
                         TokenStream(
                             [],
                         ),
                     ),
                     Token(
                         Token {
                             kind: Semi,
                             span: ./someany/test.rs:8:39: 8:40 (#0),
                         },
                         Alone,
                     ),
                     Token(
                         Token {
                             kind: Ident(
                                 "println",
                                 No,
                             ),
                             span: ./someany/test.rs:9:5: 9:12 (#0),
                         },
                         Joint,
                     ),
                     Token(
                         Token {
                             kind: Bang,
                             span: ./someany/test.rs:9:12: 9:13 (#0),
                         },
                         JointHidden,
                     ),
                     Delimited(
                         DelimSpan {
                             open: ./someany/test.rs:9:13: 9:14 (#0),
                             close: ./someany/test.rs:9:23: 9:24 (#0),
                         },
                         DelimSpacing {
                             open: JointHidden,
                             close: Joint,
                         },
                         Parenthesis,
                         TokenStream(
                             [
                                 Token(
                                     Token {
                                         kind: Literal(
                                             Lit {
                                                 kind: Str,
                                                 symbol: "{:?}",
                                                 suffix: None,
                                             },
                                         ),
                                         span: ./someany/test.rs:9:14: 9:20 (#0),
                                     },
                                     Joint,
                                 ),
                                 Token(
                                     Token {
                                         kind: Comma,
                                         span: ./someany/test.rs:9:20: 9:21 (#0),
                                     },
                                     Alone,
                                 ),
                                 Token(
                                     Token {
                                         kind: Ident(
                                             "v",
                                             No,
                                         ),
                                         span: ./someany/test.rs:9:22: 9:23 (#0),
                                     },
                                     JointHidden,
                                 ),
                             ],
                         ),
                     ),
                     Token(
                         Token {
                             kind: Semi,
                             span: ./someany/test.rs:9:24: 9:25 (#0),
                         },
                         Alone,
                     ),
                 ],
             ),
         ),
     ],
 )

