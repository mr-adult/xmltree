//! A simple library for parsing an XML file into an in-memory tree structure
//!
//! Not recommended for large XML files, as it will load the entire file into memory.
//!
//! # Example
//!
//! ```no_run
//! use xmltree::Element;
//! use std::fs::File;
//!
//! let data: &'static str = r##"
//! <?xml version="1.0" encoding="utf-8" standalone="yes"?>
//! <names>
//!     <name first="bob" last="jones" />
//!     <name first="elizabeth" last="smith" />
//! </names>
//! "##;
//!
//! let mut names_element = Element::parse(data.as_bytes()).unwrap();
//!
//! println!("{:#?}", names_element);
//! {
//!     // get first `name` element
//!     let name = names_element.get_mut_child("name").expect("Can't find name element");
//!     name.attributes.insert("suffix".to_owned(), "mr".to_owned());
//! }
//! names_element.write(File::create("result.xml").unwrap());
//!
//!
//! ```

#[cfg(all(feature = "attribute-order", not(feature = "attribute-sorted")))]
/// The type used to store element attributes.
pub type AttributeMap<K, V> = indexmap::map::IndexMap<K, V>;
#[cfg(all(feature = "attribute-sorted", not(feature = "attribute-order")))]
/// The type used to store element attributes.
pub type AttributeMap<K, V> = std::collections::BTreeMap<K, V>;
// When both features disabled or both enabled, use a fallback so irrelevant compiler errors don't
// appear…
#[cfg(any(
    not(any(feature = "attribute-sorted", feature = "attribute-order")),
    all(feature = "attribute-order", feature = "attribute-sorted")
))]
/// The type used to store element attributes.
///
/// By default this is a HashMap, but this can be changed with the "attribute-sorted" or "attribute-order" features
pub type AttributeMap<K, V> = std::collections::HashMap<K, V>;
// But don't let the invalid case off easy, now that we've made sure this is the only compiler
// error they'll see.
#[cfg(all(feature = "attribute-order", feature = "attribute-sorted"))]
compile_error!("`attribute-order` and `attribute-sorted` are mutually exclusive — pick one");

use std::borrow::Cow;
use std::fmt;
use std::io::Read;

use tree_iterators_rs::fallible_tree_collection_iterators::{
    FallibleTreeCollectionIterator, FallibleTreeCollectionIteratorBase,
};
pub use xml::namespace::Namespace;
pub use xml::reader::ParserConfig;
use xml::reader::{EventReader, XmlEvent};
pub use xml::writer::{EmitterConfig, Error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XMLNode {
    Element(Element),
    Comment(String),
    CData(String),
    Text(String),
    ProcessingInstruction(String, Option<String>),
}

trait AttributeMapExt {
    fn allocate(capacity: usize) -> Self;
}

#[cfg(feature = "attribute-sorted")]
impl<K: Ord, V> AttributeMapExt for AttributeMap<K, V> {
    fn allocate(_capacity: usize) -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "attribute-sorted"))]
impl<K, V> AttributeMapExt for AttributeMap<K, V> {
    fn allocate(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }
}

impl XMLNode {
    pub fn as_element(&self) -> Option<&Element> {
        if let XMLNode::Element(e) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn as_mut_element(&mut self) -> Option<&mut Element> {
        if let XMLNode::Element(e) = self {
            Some(e)
        } else {
            None
        }
    }
    pub fn as_comment(&self) -> Option<&str> {
        if let XMLNode::Comment(c) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn as_cdata(&self) -> Option<&str> {
        if let XMLNode::CData(c) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn as_text(&self) -> Option<&str> {
        if let XMLNode::Text(c) = self {
            Some(c)
        } else {
            None
        }
    }
    pub fn as_processing_instruction(&self) -> Option<(&str, Option<&str>)> {
        if let XMLNode::ProcessingInstruction(s, o) = self {
            Some((s, o.as_ref().map(|s| s.as_str())))
        } else {
            None
        }
    }
}

/// Represents an XML element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// This elements prefix, if any
    pub prefix: Option<String>,

    /// This elements namespace, if any
    pub namespace: Option<String>,

    /// The full list of namespaces, if any
    ///
    /// The `Namespace` type is exported from the `xml-rs` crate.
    pub namespaces: Option<Namespace>,

    /// The name of the Element.  Does not include any namespace info
    pub name: String,

    /// The Element attributes
    ///
    /// By default, this is a `HashMap`, but there are two optional features that can change this:
    ///
    /// * If the "attribute-order" feature is enabled, then this is an [IndexMap](https://docs.rs/indexmap/2/indexmap/),
    ///   which will retain item insertion order.
    /// * If the "attribute-sorted" feature is enabled, then this is a [`std::collections::BTreeMap`], which maintains keys in sorted order.
    pub attributes: AttributeMap<String, String>,
}

/// Errors that can occur parsing XML
#[derive(Debug)]
pub enum ParseError {
    /// The XML is invalid
    MalformedXml(xml::reader::Error),
    /// This library is unable to process this XML. This can occur if, for
    /// example, the XML contains processing instructions.
    CannotParse,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseError::MalformedXml(ref e) => write!(f, "Malformed XML. {}", e),
            ParseError::CannotParse => write!(f, "Cannot parse"),
        }
    }
}

impl std::error::Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::MalformedXml(..) => "Malformed XML",
            ParseError::CannotParse => "Cannot parse",
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            ParseError::MalformedXml(ref e) => Some(e),
            ParseError::CannotParse => None,
        }
    }
}

impl Element {
    /// Create a new empty element with given name
    ///
    /// All other fields are empty
    pub fn new(name: &str) -> Element {
        Element {
            name: String::from(name),
            prefix: None,
            namespace: None,
            namespaces: None,
            attributes: AttributeMap::new(),
        }
    }

    /// Parses some data into a list of `XMLNode`s
    ///
    /// This is useful when you want to capture comments or processing instructions that appear
    /// before or after the root node
    pub fn parse_all<R: Read>(r: R) -> XMLTreeIterator<R> {
        let parser_config = ParserConfig::new().ignore_comments(false);
        Element::parse_all_with_config(r, parser_config)
    }

    pub fn parse_all_with_config<R: Read>(r: R, parser_config: ParserConfig) -> XMLTreeIterator<R> {
        XMLTreeIterator::new(r, parser_config)
    }
}

/// A predicate for matching elements.
///
/// The default implementations allow you to match by tag name or a tuple of
/// tag name and namespace.
pub trait ElementPredicate {
    fn match_element(&self, e: &Element) -> bool;
}

// Unfortunately,
// `impl<TN> ElementPredicate for TN where String: PartialEq<TN>` and
// `impl<TN, NS> ElementPredicate for (TN, NS) where String: PartialEq<TN>, String: PartialEq<NS>`
// are conflicting implementations, even though we know that there is no
// implementation for tuples. We just manually implement `ElementPredicate` for
// all `PartialEq` impls of `String` and forward them to the 1-tuple version.
//
// This can probably be fixed once specialization is stable.
impl<TN> ElementPredicate for (TN,)
where
    String: PartialEq<TN>,
{
    fn match_element(&self, e: &Element) -> bool {
        e.name == self.0
    }
}

impl<'a> ElementPredicate for &'a str {
    /// Search by tag name
    fn match_element(&self, e: &Element) -> bool {
        (*self,).match_element(e)
    }
}

impl<'a> ElementPredicate for Cow<'a, str> {
    /// Search by tag name
    fn match_element(&self, e: &Element) -> bool {
        (&**self,).match_element(e)
    }
}

impl ElementPredicate for String {
    /// Search by tag name
    fn match_element(&self, e: &Element) -> bool {
        (&**self,).match_element(e)
    }
}

impl<TN, NS> ElementPredicate for (TN, NS)
where
    String: PartialEq<TN>,
    String: PartialEq<NS>,
{
    /// Search by a tuple of (tagname, namespace)
    fn match_element(&self, e: &Element) -> bool {
        e.name == self.0
            && e.namespace
                .as_ref()
                .map(|ns| ns == &self.1)
                .unwrap_or(false)
    }
}

pub struct XMLTreeIterator<R: Read> {
    reader: EventReader<R>,
    open_element_names: Vec<String>,
    prune_stack: Vec<bool>,
    path: Vec<usize>,
    errored: bool,
    moved: bool,
}

impl<R: Read> XMLTreeIterator<R> {
    fn new(r: R, parser_config: ParserConfig) -> Self {
        Self {
            reader: EventReader::new_with_config(r, parser_config),
            open_element_names: Vec::new(),
            prune_stack: Vec::new(),
            path: Vec::new(),
            errored: false,
            moved: false,
        }
    }
}

impl<R: Read> Iterator for XMLTreeIterator<R> {
    type Item = Result<XMLNode, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored {
            return None;
        }

        loop {
            match self.reader.next() {
                Ok(XmlEvent::EndElement { ref name }) => {
                    self.moved = true;

                    if Some(&name.local_name) == self.open_element_names.last() {
                        self.open_element_names.pop();
                        self.prune_stack.pop();
                        if self.open_element_names.len() + 1 < self.path.len() {
                            self.path.pop();
                        }
                    } else {
                        self.errored = true;
                        return Some(Err(ParseError::CannotParse));
                    }
                }
                Ok(XmlEvent::StartElement {
                    name,
                    attributes,
                    namespace,
                }) => {
                    self.moved = true;

                    let mut attr_map = AttributeMap::new();
                    for attr in attributes {
                        attr_map.insert(attr.name.local_name, attr.value);
                    }

                    let new_elem = Element {
                        prefix: name.prefix,
                        namespace: name.namespace,
                        namespaces: if namespace.is_essentially_empty() {
                            None
                        } else {
                            Some(namespace)
                        },
                        name: name.local_name.clone(),
                        attributes: attr_map,
                    };

                    self.open_element_names.push(name.local_name);
                    self.prune_stack.push(false);
                    while self.path.len() > self.open_element_names.len() + 1 {
                        self.path.pop();
                    }

                    if self.path.len() < self.open_element_names.len() {
                        self.path.push(0);
                    }
                    if let Some(last_path_segment) = self.path.last_mut() {
                        *last_path_segment += 1;
                    }

                    if !self.prune_stack.iter().any(|pruned| *pruned) {
                        return Some(Ok(XMLNode::Element(new_elem)));
                    }
                }
                Ok(XmlEvent::Characters(s)) => {
                    self.moved = true;

                    self.path.push(0);
                    if !self.prune_stack.iter().any(|pruned| *pruned) {
                        return Some(Ok(XMLNode::Text(s)));
                    }
                }
                Ok(XmlEvent::Whitespace(..)) => {
                    self.moved = true;
                }
                Ok(XmlEvent::Comment(s)) => {
                    self.moved = true;

                    self.path.push(0);
                    if !self.prune_stack.iter().any(|pruned| *pruned) {
                        return Some(Ok(XMLNode::Comment(s)));
                    }
                }
                Ok(XmlEvent::CData(s)) => {
                    self.moved = true;

                    self.path.push(0);
                    if !self.prune_stack.iter().any(|pruned| *pruned) {
                        return Some(Ok(XMLNode::CData(s)));
                    }
                }
                Ok(XmlEvent::ProcessingInstruction { name, data }) => {
                    self.moved = true;

                    self.path.push(0);
                    if !self.prune_stack.iter().any(|pruned| *pruned) {
                        return Some(Ok(XMLNode::ProcessingInstruction(name, data)));
                    }
                }
                Ok(XmlEvent::StartDocument { .. }) => {
                    if self.moved {
                        return Some(Err(ParseError::CannotParse));
                    }

                    self.moved = true;
                }
                Ok(XmlEvent::EndDocument) => {
                    return None;
                }
                Err(e) => {
                    self.moved = true;

                    self.errored = true;
                    return Some(Err(ParseError::MalformedXml(e)));
                }
            }
        }
    }
}

impl<R: Read> FallibleTreeCollectionIteratorBase<XMLNode, (), ParseError> for XMLTreeIterator<R> {
    fn current_path(&self) -> &[usize] {
        &self.path
    }

    fn prune_current_subtree(&mut self) {
        if let Some(top) = self.prune_stack.last_mut() {
            *top = true;
        }
    }
}

impl<R: Read> FallibleTreeCollectionIterator<XMLNode, (), ParseError> for XMLTreeIterator<R> {}
