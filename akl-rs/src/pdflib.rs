use thiserror::Error;

// Color handling
use colorsys::Rgb;

// low level pdf library
use lopdf::dictionary;
use lopdf::{Document, Dictionary, Object, ObjectId};

// standard library tools
use std::collections::HashMap;
use std::path::Path;
use chrono::Datelike;

use sha2::{Digest,Sha256};

/// PdfLibError enumerates all possible errors returned by this library.
#[derive(Error, Debug)]
pub enum PdfLibError {
    #[error("Invalid page_id found in the document")]
    InvalidPageId,

    #[error("Invalid annotation found in the document")]
    InvalidAnnotation,

    /// Invalid UTF-8 found when parsing a text-string object.
    #[error("Invalid UTF-8 byte sequence when reading 'text-string' object")]
    NDConvError { source: std::string::FromUtf8Error },

    /// Invalid UTF-16 found when parsing a text-string object.
    #[error("Invalid UTF-8 byte sequence when reading a 'text-string' object")]
    NDConvError16 { source: std::string::FromUtf16Error },

    /// Represents all other cases of `lopdf::Error`.
    #[error(transparent)]
    PDFError(#[from] lopdf::Error),

    /// Represents all other cases of `lopdf::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}


// TODO:
// (5) Import des annotations depuis une copie du document (utile!!!)
//      -> this is almost already possible 
//
// (6) Compute the hash of the document
//

#[derive(Debug,Clone)]
pub struct NamedDestination {
    /// absolute x position on the page, from the left.
    left: f32,
    /// absolute y position on the page, from the top.
    top : f32,
    /// page containing the annotation.
    page: ObjectId,
    /// page number
    pub page_num: u32,
    /// name of the annotation for external links.
    pub name: String
}

#[derive(Debug,Clone)]
struct RectangleObject {
    /// absolute x position of the lower left corner.
    x_ll : f32,
    /// absolute y position of the lower left corner.
    y_ll : f32,
    /// absolute x position of the upper right corner.
    x_ur : f32,
    /// absolute y position of the upper right corner.
    y_ur : f32,
    /// RGB fill colour of the rectangle.
    colour : Rgb
}

//// Generic Pdf utils 

/// Parses a "text string" object as defined by the PDF standard.
///
/// Either it is a usual PDFEncoding, or UTF8, or UTF16, depending
/// on the BOM at the start of the string.
///
/// UTF-16_BE -> \x254\x255
/// UTF-16_LE -> \x255\x254
/// UTF-8     -> \x239\x187\x191
fn parse_text_string(s : &[u8]) -> Result<String,PdfLibError> {
    if s.len() < 2 {
        String::from_utf8(s.into())
            .map_err(|e| PdfLibError::NDConvError { source : e })
    } else if s[0] == 0xfe && s[1] == 0xff { 
        let t16 : Vec<u16> = s.chunks(2)
                   .skip(1)
                   .map(|x| (x[0] as u16) << 8 | x[1] as u16).collect();
        String::from_utf16(&t16)
            .map_err(|e| PdfLibError::NDConvError16 { source : e })
    } else if s[0] == 0xff && s[1] == 0xfe { 
        let t16 : Vec<u16> = s.chunks(2)
                   .skip(1)
                   .map(|x| (x[1] as u16) << 8 | x[0] as u16).collect();
        String::from_utf16(&t16)
            .map_err(|e| PdfLibError::NDConvError16 { source : e })
    } else {
        String::from_utf8(s.into())
            .map_err(|e| PdfLibError::NDConvError { source : e })
    }
}

/// Produces the PdfObjects to draw a link with the given url
/// represented in the pdf using a borderless filled rectangle.
fn rectangle_link(rect : &RectangleObject, url : String) -> Vec<Object> {
    let rct = vec![rect.x_ll.into(),
                   rect.y_ll.into(),
                   rect.x_ur.into(),
                   rect.y_ur.into()];
    let brd = vec![0.into(), 0.into(), 0.into()];
    let clr = vec![Object::Real((rect.colour.red()   / 255.0) as f32),
                   Object::Real((rect.colour.green() / 255.0) as f32),
                   Object::Real((rect.colour.blue()  / 255.0) as f32)];
    vec![Object::Dictionary(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Link",
            "Rect" => rct.clone(),
            "Border" => brd.clone(),
            "A" => dictionary! {
                "S"    => "URI",
                "Type" => "Action",
                "URI"  => Object::string_literal(url)
            }
        }),
        Object::Dictionary(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Square",
            "Rect" => rct.clone(),
            "Border" => brd.clone(),
            "IC" => clr
        })
    ]
}

/// Converts an object to a string if it is a pdf name or a pdf string.
///
/// This is useful because PDF named destinations have names that can
/// either be represented as Strings ore Pdf Names depending on the document
/// version.
fn as_name_or_str<'a>(obj : &'a Object) -> Result<&'a [u8], PdfLibError> {
    match obj {
        Object::Name(ref name) => Ok(name) ,
        Object::String(ref name, _) => Ok(name) ,
        _ => { Err(PdfLibError::PDFError(lopdf::Error::Type)) }
    }
}

/// Tries to parse a document object
/// representing a named destination into an array of values.
///
/// It is either a Dict with a key D representing an array
/// or it is directly an array.
fn array_of_named_dest_obj<'a>(doc : &'a Document,
                               obj : &'a Object) -> Result<&'a Vec<Object>, PdfLibError>
{
    Ok(obj.as_dict()
       .and_then(move |d| d.get_deref(b"D", doc))
       .or_else(|_| Ok(obj))
       .and_then(Object::as_array)?)
}

/// Parses a named destination pdf object into a
/// NamedDestination structure. The full document is needed
/// to follow indirect objects in the pdf.
///
/// Named destinations 12.3.2.3 of the pdf 1.7 document reference
/// states that it can either be an array, or an object with key D
/// being an array. The values of the array are specified in Table 151.
fn named_dest_of_object(doc : &Document,
                        pnum: &HashMap<ObjectId, u32>,
                        key : &Object,
                        obj : &Object,
) -> Result<NamedDestination,PdfLibError> {
    let name = parse_text_string(as_name_or_str(key)?)?;

    let mut top  : f32 = 10.0;
    let mut left : f32 = 10.0;
    let mut m_page = Err(PdfLibError::InvalidPageId);

    // First we follow the links to get the "real" object
    let true_obj = doc.dereference(obj).map(|(_,o)| o)?;

    let arr : &Vec<Object> = array_of_named_dest_obj(doc, true_obj)?;

    if arr.len() > 1 {
        m_page = arr[0].as_reference().map_err(PdfLibError::PDFError);
        let dest_type = arr[1].as_name()?;
        if arr.len() > 3 && dest_type == b"XYZ" {
            left = arr[2].as_float().unwrap_or(left);
            top  = arr[3].as_float().unwrap_or(top);
        }
    }

    let page = m_page?;
    let page_num = *pnum.get(&page).ok_or(PdfLibError::InvalidPageId)?;

    Ok(NamedDestination {
        left,
        top,
        page,
        name,
        page_num,
    })
}

/// Iterate over a name tree as described
/// in the PDF documentation
fn name_tree_iter<'a>(doc : &'a Document, tree: &'a Dictionary)
    -> Box<dyn Iterator<Item = &'a [Object]> + 'a> {
    // If we have kids, then there are no names and we recursively iterate
    if let Ok(kids) = tree.get(b"Kids").and_then(Object::as_array) {
        Box::new(kids.iter().flat_map(|kid| {
            if let Ok(kid) = doc.dereference(kid)
                                .map(|(_,obj)| obj)
                                .and_then(Object::as_dict) {
                name_tree_iter(doc, kid)
            } else {
                Box::new(std::iter::empty())
            }
        }))
    // otherwise, we may be a leaf with names, and we produce the correct output
    } else if let Ok(names) = tree.get(b"Names").and_then(Object::as_array) {
        Box::new(
            names.chunks_exact(2)
        )
    // this may not be an error according to the spec ...
    } else {
        Box::new(std::iter::empty())
    }
}

/// Iterate over a number tree as described
/// in the PDF documentation section 7.9.7
#[allow(dead_code)]
fn number_tree_iter<'a>(doc : &'a Document, tree: &'a Dictionary)
    -> Box<dyn Iterator<Item = &'a [Object]> + 'a> {
    // If we have kids, then there are no names and we recursively iterate
    if let Ok(kids) = tree.get(b"Kids").and_then(Object::as_array) {
        Box::new(kids.iter().flat_map(|kid| {
            if let Ok(kid) = doc.dereference(kid)
                                .map(|(_,obj)| obj)
                                .and_then(Object::as_dict) {
                name_tree_iter(doc, kid)
            } else {
                Box::new(std::iter::empty())
            }
        }))
    // otherwise, we may be a leaf with names, and we produce the correct output
    } else if let Ok(names) = tree.get(b"Nums").and_then(Object::as_array) {
        Box::new(
            names.chunks_exact(2)
        )
    // this may not be an error according to the spec ...
    } else {
        Box::new(std::iter::empty())
    }
}

/// Fetch the named destinations of a given PDF document.
///
/// FIXME: for pdf 1.1 documents this was directly found as a
/// reference to a dict located at ``/Root/Dests``. 
fn collect_named_destinations(pdf : &Document, pnum: &HashMap<ObjectId,u32>)
    -> Result<Vec<NamedDestination>, PdfLibError> {
    let catalog = pdf.catalog()?;
    // pdf 1.1 named destinations in a simple dict
    let old_dests = catalog.get_deref(b"Dests", pdf).and_then(Object::as_dict);
    // pdf 1.2 named destinations in a name tree object
    let new_dests = catalog.get_deref(b"Names", pdf)
                           .and_then(Object::as_dict)
                           .and_then(|nms| nms.get_deref(b"Dests", pdf))
                           .and_then(Object::as_dict);

    // prefer the newer versions
    if let Ok(dests) = new_dests {
        name_tree_iter(pdf, dests).map(|key_val|
            named_dest_of_object(pdf, pnum, &key_val[0], &key_val[1])
        ).collect()
    // fallback for old documents
    } else if let Ok(dests) = old_dests {
        dests.into_iter().map(|(k,v)| {
            named_dest_of_object(pdf, pnum, &Object::Name(k.as_slice().to_vec()), v)
        }).collect()
    // It is not a problem if such a dict does not exist!
    // we should not fail.
    } else {
        Ok(vec![])
    }
}



/// Iterate over the annotations that appear in a document
/// we assume that annotations are always given as indirect objects
/// (which I think is standard in pdf documents)
fn page_annotations_iter<'a>(pdf: &'a Document) -> impl Iterator<Item = ObjectId> + 'a {
    // iterate over the pages to get the arrays of annotations
    pdf.page_iter().flat_map(move |page_id| {
        let page_obj = pdf.get_dictionary(page_id)?;
        page_obj.get_deref(b"Annots", pdf)
                .and_then(Object::as_array)
    // only select those objects that are indirect
    }).flat_map(|page_ans| {
        page_ans.iter().flat_map(Object::as_reference)
    })
}

/// Appends annotation objets to a given page.
/// The objects should probably be indirect references
/// to previously added objets.
fn append_annots_to_page(pdf : &mut Document,
                         page_id : ObjectId,
                         elts: &mut Vec<Object>)
-> Result<(), PdfLibError> {
    let page = pdf.get_dictionary_mut(page_id)?;
    // if no array is present, create one
    if !page.has(b"Annots") {
        page.set(b"Annots".to_owned(), vec![]);
    }
    match page.get(b"Annots")? {
        // First case: the array is direct
        Object::Array(_) => {
            let arr = page.get_mut(b"Annots")
                .and_then(Object::as_array_mut)?;
            Ok(arr.append(elts))
        }
        // Second case: the array is indirect
        Object::Reference(_) => {
            let arr = page.get(b"Annots")
                                .and_then(Object::as_reference)
                                .and_then(|k| pdf.get_object_mut(k))
                                .and_then(Object::as_array_mut)?;
            Ok(arr.append(elts))
        }
        // otherwise, we do not have a correct annotation array
        _ => {
            Err(PdfLibError::InvalidAnnotation)
        }
    }
}


/// Update the URL of one link according to the update function.
fn update_link<F>(dct : &mut Dictionary, lik : &F) -> Result<(), PdfLibError>
    where 
        F : Fn(String) -> String
{
    let action : &mut Dictionary = dct.get_mut(b"A").and_then(Object::as_dict_mut)?;
    if let Ok(raw_uri) = action.get(b"URI").and_then(Object::as_str) {
        let old_uri = parse_text_string(raw_uri)?;
        action.set("URI",
                   lopdf::Object::String(
                        lik(old_uri).into(),
                        lopdf::StringFormat::Literal
                   )
            );
    }
    Ok(())
}

#[derive(Debug,Clone)]
pub struct PdfMetaData {
    /// Potential title of the pdf file.
    pub title       : Option<String>,
    /// Additional context of the pdf (publisher, conference, etc.)
    pub context     : Vec<String>,
    /// Authors of the pdf file.
    pub authors     : Vec<String>,
    /// Publication year of the pdf file.
    pub year        : Option<u32>,
    /// Identifiers found inside the pdf (arxiv, doi, ISBN, etc.)
    pub identifiers : Vec<String>,
}



//// MUTABILITY ////


#[derive(Debug,Clone)]
/// A wrapper around the notion of pdf document.
pub struct PdfDocument {
    /// The inner document represented in memory.
    pdf         : Document,
    /// Hash map to convert between page ids and page numbers.
    /// in the pdf document.
    //page_nums   : HashMap<ObjectId, u32>,
    /// Named destinations of the inner pdf.
    named_dests : Vec<NamedDestination>,
    /// All the annotations that can be found in the document.
    annotations : Vec<ObjectId>,
}

impl TryFrom<Document> for PdfDocument {
    type Error = PdfLibError;
    fn try_from(value: Document) -> Result<Self, Self::Error> {
        // Collect the pages and their respective numbers 
        let mut page_nums = HashMap::new();
        value.page_iter()
             .enumerate()
             .for_each(|(i, page_id)| {
                 page_nums.insert(page_id, (i+1) as u32);
             });
        // Collect the named destinations in some suitable vector
        let named_dests = collect_named_destinations(&value, &page_nums)?;

        // Collect all the annotations present in the document
        let annotations = page_annotations_iter(&value).collect();

        Ok(PdfDocument {
            pdf: value,
            named_dests,
            annotations,
            //page_nums,
        })
    }
}



impl PdfDocument {

    /// Provides a checksum of the pdf contents
    pub fn get_checksum(&mut self) -> Result<String, PdfLibError> {
        let mut hasher = Sha256::new();
        self.pdf.save_to(&mut hasher)?;
        let checksum = hasher.finalize();
        Ok(format!("{:x}", checksum))
    }



    /// Extract Meta Data from the /Info field
    /// and the /Metadata XMP metadata if
    /// it exists.
    ///
    /// TODO: fetch the XMP field 
    /// /Root /Metadata -> XMP Stream 
    ///
    /// In particular, dc_creator for the list of authors
    ///                dc_identifier for the unique identifier
    ///                dc_title
    pub fn get_meta_data(&self) -> Result<PdfMetaData, PdfLibError>
    {
        let pdf = &self.pdf;
        let infos = pdf.trailer.get_deref(b"Info", pdf)
                               .and_then(Object::as_dict)?;
        let title = infos.get(b"Title")
                         .and_then(Object::as_str)
                         .map_err(|e| PdfLibError::PDFError(e))
                         .and_then(parse_text_string).ok();
        // In the pdf meta-data ... only one author a priori :(
        let authors : Vec<String>
            = infos.get(b"Author")
                   .and_then(Object::as_str)
                   .map_err(|e| PdfLibError::PDFError(e))
                   .and_then(parse_text_string)
                   .map(|s| s.split(',')
                              .map(|e| e.trim())
                              .map(String::from)
                              .collect())
                   .unwrap_or(vec![]);
        let year : Option<u32> = 
            infos.get(b"CreationDate")
                 .ok()
                 .and_then(Object::as_datetime)
                 .and_then(|d| d.year().try_into().ok());

        let context = vec![];
        let identifiers = vec![];

        Ok(PdfMetaData {
            title,
            authors,
            context,
            year,
            identifiers,
        })
    }


    /// Save the pdf to a given file.
    pub fn save_to(&mut self, path : &Path) 
        -> Result<std::fs::File,PdfLibError> {
        Ok(self.pdf.save(path)?)
    }


    /// Add rectangle links around the named destinations,
    /// using the closure to build the external URLs.
    pub fn add_destinations_links<F>(&mut self, lik : F) -> Result<(), PdfLibError>
        where 
            F : Fn(NamedDestination) -> String
    {
        // temporary rectangle object
        let mut rect = RectangleObject {
            x_ll : 0.0, y_ll : 0.0, x_ur : 0.0, y_ur : 0.0,
            colour : Rgb::from_hex_str("8FBCBB").unwrap(),
        };
        // what should be added to the pages
        let mut page_annots : HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();

        // creates all the objects in the pdf document
        self.named_dests.iter().for_each(|destination| {
            rect.x_ll = destination.left - 10.0;
            rect.x_ur = destination.left - 5.0;
            rect.y_ll = destination.top - 10.0;
            rect.y_ur = destination.top - 5.0;

            let mut ids = rectangle_link(&rect, lik(destination.clone()))
                          .iter()
                          .map(|obj| self.pdf.add_object(obj.clone()))
                          .collect();

            page_annots.entry(destination.page)
                       .or_insert(vec![])
                       .append(&mut ids);
        });

        // batch addition of the objects to the respective pages
        page_annots.iter_mut().map(|(k,v)| {
            let mut objs : Vec<Object> = v.iter()
                .map(|&x| Object::Reference(x)).collect();
            self.annotations.append(v);
            append_annots_to_page(&mut self.pdf, *k, &mut objs)
        }).collect()
    }

    /// Updates all external URL links inside the pdf document.
    pub fn update_links<F>(&mut self, lik : &F) -> Result<(), PdfLibError>
        where 
            F : Fn(String) -> String
    {
        for &annot in &self.annotations {
            let mut_obj = self.pdf.get_object_mut(annot)
                              .and_then(Object::as_dict_mut)?;
            // We do not care if this operation fails
            update_link(mut_obj, lik).unwrap_or(());
        }
        Ok(())
    }
}


//pub fn test_functions () {
    //let pdf = Document::load(Path::new("manuscript.pdf")).unwrap();
    //let mut doc = PdfDocument::try_from(pdf).unwrap();

    //doc.update_links(&|s| s).unwrap();

    //doc.add_destinations_links(|s| {
        //format!("{} page {}", s.name, s.page_num)
    //}).unwrap();

    //doc.pdf.save(PathBuf::from(r"output.pdf")).unwrap();
//}
