extern crate xmltree;
use tree_iterators_rs::prelude::*;
use std::fs::File;
use xmltree::*;

#[test]
fn test_01() {
    let e = Element::parse_all(File::open("tests/data/01.xml").unwrap())
        .prune(|node| {
            if let XMLNode::Element(el) = node {
                el.name == "libraries"
            } else {
                false
            }
        })
        .collect_trees()
        .unwrap();
    println!("{e:#?}");
}

#[test]
fn test_02() {
    let e = Element::parse_all(File::open("tests/data/02.xml").unwrap())
        .collect_trees()
        .unwrap();
    println!("{:#?}", e);
}

#[test]
fn test_03() {
    let e = Element::parse_all(File::open("tests/data/03.xml").unwrap())
        .collect_trees()
        .unwrap();
    println!("{:#?}", e);
}

#[test]
fn test_04() {
    let e = Element::parse_all(File::open("tests/data/04.xml").unwrap())
        .collect_trees()
        .unwrap();
    println!("{:#?}", e);

    if let XMLNode::ProcessingInstruction(pt1, pt2) = &e[1].children[0].value {
        assert_eq!(pt1, "pi");
        assert_eq!(pt2.as_ref().unwrap(), "foo=\"blah\"");
    } else {
        panic!("Should be processing instruction");
    }
}

#[test]
fn test_no_root_node() {
    let result = Element::parse_all(File::open("tests/data/05.xml").unwrap()).collect_trees();
    assert!(result.is_err())
}

#[test]
fn test_mal_01() {
    // some tests for error handling

    let data = r#"
        <?xml version="1.0" encoding="utf-8" standalone="yes"?>
        <names>
            <name first="bob" last="jones />
            <name first="elizabeth" last="smith" />
        </names>
    "#;

    let names_element = Element::parse_all(data.as_bytes()).collect_trees();
    if let Err(ParseError::MalformedXml(..)) = names_element {
        // OK
    } else {
        panic!("unexpected parse result");
    }
    println!("{:?}", names_element);
}

#[test]
fn test_mal_02() {
    // some tests for error handling

    let data = r##"
            this is not even close
            to XML
    "##;

    let names_element = Element::parse_all(data.as_bytes()).collect_trees();
    if let Err(ParseError::MalformedXml(..)) = names_element {
        // OK
    } else {
        panic!("unexpected parse result");
    }
    println!("{:?}", names_element);
}

#[test]
fn test_mal_03() {
    // some tests for error handling

    let data = r#"
        <?xml version="1.0" encoding="utf-8" standalone="yes"?>
        <names>
            <name first="bob" last="jones"></badtag>
            <name first="elizabeth" last="smith" />
        </names>
    "#;

    let names_element = Element::parse_all(data.as_bytes()).collect_trees();
    if let Err(ParseError::MalformedXml(..)) = names_element {
        // OK
    } else {
        panic!("unexpected parse result");
    }
    println!("{:?}", names_element);
}

// #[test]
// fn test_take() {
//     let data_xml_1 = r#"
//         <?xml version="1.0" encoding="utf-8" standalone="yes"?>
//         <names>
//             <name first="bob" last="jones"></name>
//             <name first="elizabeth" last="smith" />
//             <remove_me key="value">
//                 <child />
//             </remove_me>
//         </names>
//     "#;
// 
//     let data_xml_2 = r#"
//         <?xml version="1.0" encoding="utf-8" standalone="yes"?>
//         <names>
//             <name first="bob" last="jones"></name>
//             <name first="elizabeth" last="smith" />
//         </names>
//     "#;
// 
//     let mut data_1 = Element::parse_all(data_xml_1.trim().as_bytes()).unwrap();
//     let data_2 = Element::parse(data_xml_2.trim().as_bytes()).unwrap();
// 
//     if let Some(removed) = data_1.take_child("remove_me") {
//         assert_eq!(removed.children.len(), 1);
//     } else {
//         panic!("take_child failed");
//     }
// 
//     assert_eq!(data_1, data_2);
// }
// 
// #[test]
// fn test_ns_rw() {
//     {
//         let e: Element = Element::parse(File::open("tests/data/ns1.xml").unwrap()).unwrap();
// 
//         let mut buf = Vec::new();
//         e.write(&mut buf).unwrap();
// 
//         let e2 = Element::parse(Cursor::new(buf)).unwrap();
// 
//         assert_eq!(e, e2);
//     }
//     {
//         let e: Element = Element::parse(File::open("tests/data/ns2.xml").unwrap()).unwrap();
// 
//         let mut buf = Vec::new();
//         e.write(&mut buf).unwrap();
// 
//         let e2 = Element::parse(Cursor::new(buf)).unwrap();
// 
//         assert_eq!(e, e2);
//     }
// }
// 
// #[test]
// fn test_write_with_config() {
//     let e: Element = Element::parse(File::open("tests/data/01.xml").unwrap()).unwrap();
// 
//     let cfg = EmitterConfig {
//         perform_indent: true,
//         ..EmitterConfig::default()
//     };
// 
//     let mut buf = Vec::new();
//     e.write_with_config(&mut buf, cfg).unwrap();
// 
//     let s = String::from_utf8(buf).unwrap();
//     println!("{}", s);
// }
// 
// #[test]
// fn test_ns() {
//     let e: Element = Element::parse(File::open("tests/data/ns1.xml").unwrap()).unwrap();
// 
//     let htbl = e
//         .get_child(("table", "http://www.w3.org/TR/html4/"))
//         .unwrap();
//     let ftbl = e
//         .get_child(("table", "https://www.w3schools.com/furniture"))
//         .unwrap();
// 
//     assert_ne!(htbl, ftbl);
// }
// 
// #[test]
// fn test_text() {
//     let data = r#"
//         <?xml version="1.0" encoding="utf-8" standalone="yes"?>
//         <elem><inner/></elem>
//     "#;
// 
//     let elem = Element::parse(data.trim().as_bytes()).unwrap();
//     assert!(elem.get_text().is_none());
// 
//     let data = r#"
//         <?xml version="1.0" encoding="utf-8" standalone="yes"?>
//         <elem>hello world<inner/></elem>
//     "#;
// 
//     let elem = Element::parse(data.trim().as_bytes()).unwrap();
//     assert_eq!(elem.get_text().unwrap(), Cow::Borrowed("hello world"));
// 
//     let data = r#"
//         <?xml version="1.0" encoding="utf-8" standalone="yes"?>
//         <elem>hello <inner/>world</elem>
//     "#;
// 
//     let elem = Element::parse(data.trim().as_bytes()).unwrap();
//     assert_eq!(
//         elem.get_text().unwrap(),
//         Cow::from("hello world".to_owned())
//     );
// 
//     let data = r#"
//         <?xml version="1.0" encoding="utf-8" standalone="yes"?>
//         <elem>hello <inner/><![CDATA[<world>]]></elem>
//     "#;
// 
//     let elem = Element::parse(data.trim().as_bytes()).unwrap();
//     assert_eq!(
//         elem.get_text().unwrap(),
//         Cow::from("hello <world>".to_owned())
//     );
// }
// 
// #[test]
// fn test_nodecl() {
//     let mut c = EmitterConfig::new();
//     c.write_document_declaration = false;
//     let e = Element::new("n");
//     let mut output = Vec::new();
//     e.write_with_config(&mut output, c).unwrap();
//     assert_eq!(String::from_utf8(output).unwrap(), "<n />");
// }
// 
// #[test]
// fn test_decl() {
//     let mut c = EmitterConfig::new();
//     c.write_document_declaration = true;
//     let e = Element::new("n");
//     let mut output = Vec::new();
//     e.write_with_config(&mut output, c).unwrap();
//     assert_eq!(
//         String::from_utf8(output).unwrap(),
//         "<?xml version=\"1.0\" encoding=\"UTF-8\"?><n />"
//     );
// }
// 