// local directories (cross platform)
use directories::ProjectDirs;
// path handling
use std::path::PathBuf;
// hashmap 
use std::collections::HashMap;
// command line argument parsing
use clap::{Parser, Subcommand, Args};

use url::Url;

// clipboard management
// with extra x11 trickery to get the clipboard to work
use copypasta_ext::prelude::*;
use copypasta_ext::x11_bin::ClipboardContext;


// serialisation  and deserialisation 
use serde::{Serialize, Deserialize};

// Error handling in app
use anyhow::{Result, Context};

mod pdflib;
//mod view;
//mod document;
//mod commands;


/// Arguments given to a citation command.
/// The URI is typically a DOI.
#[derive(Args,Debug,Serialize,Deserialize,Clone)]
struct CiteArgs {
    /// URI to the document to be cited
    #[arg(short, long)]
    uri: String,

    /// Citation's page
    #[arg(short, long)]
    page: Option<u32>,

    /// Citation's named destination
    #[arg(short, long)]
    dest: Option<String>,

    /// From where does this link
    /// has been written (url / uid)
    #[arg(short, long)]
    from: Option<String>,
}

/// Arguments given to the import command.
/// The URI is either a filepath or a download URL,
/// that gives a direct access to the pdf document.
///
/// The additional metadata will be completed by
/// the one fetched from the PDF file, and
/// manually completed if --interactive is activated.
#[derive(Clone,Args,Debug,Serialize,Deserialize)]
struct ImportArgs {
    /// URI to the document
    #[arg(short, long)]
    uri: String,

    /// title of the document
    #[arg(short, long)]
    title: Option<String>,

    /// Authors of the document
    #[arg(short, long)]
    authors: Vec<String>,

    /// Additional context (conference, journal, etc.)
    #[arg(short, long)]
    context: Vec<String>,

    /// Identifiers (DOI, Arxiv IDs, etc)
    #[arg(short, long)]
    identifiers: Vec<String>,

    /// Publication Year
    #[arg(short, long)]
    year: Option<u32>,

    /// View after import?
    #[arg(short, long, default_value="true")]
    view: bool,

    /// Force re-import even if the pdf is in the library?
    #[arg(short, long, default_value="false")]
    force: bool,
}

/// Arguments given to the resolve command.
#[derive(Args,Debug,Serialize,Deserialize,Clone)]
struct ResolveArgs {
    /// URI to the document
    #[arg(short, long)]
    uri: String,
}


/// Arguments given to the convert command.
/// The URI must be a valid filepath to a pdf document.
///
/// This command typically is used when opening
/// a "working document".
///
/// TODO: also allow urls to be downloaded?
#[derive(Args,Debug,Serialize,Deserialize,Clone)]
struct ConvertArgs {
    /// URI to the document
    #[arg(short, long)]
    uri: String,

    /// Output file name
    #[arg(short, long)]
    output: PathBuf,
}


/// A document in the library.
#[derive(Serialize, Deserialize,Clone,Debug)]
struct Document {
    /// The SHA256 checksum of the original document
    /// seen as a string
    checksum : String,

    /// The filename of the document on the system.
    filename : String,

    /// Strings that identify this document. Typically
    /// a download URI, but it can also be a DOI or an Arxiv Link.
    ///
    /// a. Non empty vector
    /// b. Sorted by generality (DOI > Arxiv > URL > filepath)
    identifiers : Vec<String>,

    /// Understandable name of the document
    /// usually the title of a paper or a blog post.
    title : String,

    /// Authors of the document.
    authors : Vec<String>,

    /// Publication year of the document.
    year : u32,

    /// Additional context.
    /// Typically a conference name, a website name, or
    /// a working group.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    context : Vec<String>,

    /// Named destinations of the document.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    destinations : HashMap<String,Vec<String>>,
}


/// The main application state.
#[derive(Serialize, Deserialize,Clone,Debug)]
struct AppState {
    /// File path to the index.yaml file 
    /// containing the catalog of available documents.
    index_path : PathBuf,

    /// File path to the directory containing
    /// the "raw" version of the documents. 
    raw_path   : PathBuf,

    /// File path to the directory containing
    /// the "modified" version of the documents. 
    mod_path   : PathBuf,

    /// Path to the logs.
    log_path   : PathBuf,

    /// Content of the index.yaml file, parsed.
    index : Vec<Document>,
}

//// COMMAND LINE INTERFACE /////

#[derive(Parser)]
#[derive(Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Optional URI argument to execute.
    execute_uri: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Interactive flag.
    /// Uses a temporary file and the default editor to
    /// allow the user to fill out metadata.
    #[arg(short, long, default_value = "false")]
    interactive: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}


#[derive(Subcommand)]
#[derive(Debug,Clone)]
enum Commands {
    /// Cite a document (typically put a nice citation in the clipboard)
    Cite(CiteArgs),

    /// Build a "converted" document from a pdf, without storing
    /// it in the library.
    Convert(ConvertArgs),

    /// Resolve a URI to a potential filepath in the library.
    Resolve(ResolveArgs),

    /// Open a pdf document using the appropriated viewer
    /// on the system.
    ///
    /// (it turns out that the arguments are isomorphic to
    /// the cite command for now)
    View(CiteArgs),

    /// Open a document, similar to resolve followed by View.
    ///
    /// (it turns out that the arguments are isomorphic to
    /// the cite command for now)
    Open(CiteArgs),

    /// Find a document by searching current metadata.
    ///
    /// Currently only provides a list of the current pdfs
    /// suitable to be used with ROFI/FZF/Dmenu.
    Find,

    /// Imports a document into the library.
    /// (does perform a conversion)
    Import(ImportArgs),
}

#[derive(Debug,Clone)]
enum ParsedURI {
    HttpURL (String),
    DOI (String),
    Arxiv { arxiv_id : String, arxiv_version : String },
    AklCommand (Commands),
    FilePath (PathBuf),
}

/// Serialize from a command to a suitable uri
/// of the form `akl://command-name/?query-params`.
fn command_to_query(cmd : Commands) -> Result<String> {
    match cmd {
        Commands::Cite(a) => {
            let name = "cite-document";
            let params = serde_urlencoded::to_string(a)?;
            Ok(format!("akl://{name}/?{params}"))
        }
        Commands::Convert(a) => {
            let name = "convert-document";
            let params = serde_urlencoded::to_string(a)?;
            Ok(format!("akl://{name}/?{params}"))
        }
        Commands::View(a) => {
            let name = "view-document";
            let params = serde_urlencoded::to_string(a)?;
            Ok(format!("akl://{name}/?{params}"))
        }
        Commands::Open(a) => {
            let name = "open-document";
            let params = serde_urlencoded::to_string(a)?;
            Ok(format!("akl://{name}/?{params}"))
        }
        Commands::Resolve(a) => {
            let name = "resolve-document";
            let params = serde_urlencoded::to_string(a)?;
            Ok(format!("akl://{name}/?{params}"))
        }
        Commands::Import(a) => {
            let name = "import-document";
            let params = serde_urlencoded::to_string(a)?;
            Ok(format!("akl://{name}/?{params}"))
        }
        Commands::Find => {
            let name = "find-document";
            Ok(format!("akl://{name}/"))
        }
    }
}

/// Converts from a query string and command name
/// to a parsed command result.
fn query_to_command(name : &str, query : &str) -> Result<Commands> {
    match name {
        "import-document" => {
            let mut keys = serde_urlencoded::from_str::<HashMap<String,String>>(query)
                .context("Decoding the import url")?;

            let payload = keys.remove("payload")
                .context("Searching for the payload of import args")?;

            let import_args = serde_json::from_str(&payload)
                .context("Parsing the payload of the import args")?;
            Ok(Commands::Import(import_args))
        }
        "cite-document" => {
            Ok(Commands::Cite(serde_urlencoded::from_str(query)?))
        }
        "view-document" => {
            Ok(Commands::View(serde_urlencoded::from_str(query)?))
        }
        "open-document" => {
            Ok(Commands::Open(serde_urlencoded::from_str(query)?))
        }
        "resolve-document" => {
            Ok(Commands::Resolve(serde_urlencoded::from_str(query)?))
        }
        "convert-document" => {
            Ok(Commands::Convert(serde_urlencoded::from_str(query)?))
        }
        "find-document" => {
            Ok(Commands::Find)
        }
        _ => {
            anyhow::bail!("Invalid command name {name}")
        }
    }

}

fn parse_arxiv (url : Url) -> Result<ParsedURI> {
    let arxiv   = url.path();
    let version = arxiv.find("v");
    let start : Option<usize>  = 
        if &arxiv[..5] == "/abs/" ||
           &arxiv[..5] == "/pdf/" {
               Some(4)
        } else { 
               None
        };
    match (start,version) {
        (Some(s), Some(v)) => {
            Ok(ParsedURI::Arxiv { arxiv_version: arxiv[v+1..].into(),
                                  arxiv_id:  arxiv[s+1..v].into() })
        }
        (Some(s), None) => {
            Ok(ParsedURI::Arxiv { arxiv_version: "1".into(),
                                  arxiv_id:  arxiv[s+1..].into() })
        }
        (None, Some(v)) => {
            Ok(ParsedURI::Arxiv { arxiv_version: arxiv[v+1..].into(),
                                  arxiv_id:  arxiv[..v].into() })
        }
        (None,None) => {
            Ok(ParsedURI::Arxiv { arxiv_version: "1".into(),
                                  arxiv_id:  arxiv.into() })
        }
    }
}

fn parse_doi(url : Url) -> Result<ParsedURI> {
    let doi = url.path();
    match doi.chars().nth(0) {
        Some('/') => {
            Ok(ParsedURI::DOI(doi[1..].into()))
        } 
        _ => {
            Ok(ParsedURI::DOI(doi.into()))
        }
    }
}

/// URI parser
fn uri_dispatch(uri : &str) -> Result<ParsedURI> {
    let nice_url = Url::parse(uri)
        .context("URL parsing")?;

    match nice_url.scheme()  {
        "https" | "http" => {
            match nice_url.host_str() {
                Some("arxiv.org") => {
                    parse_arxiv(nice_url)
                }
                Some("doi.org") | Some("dx.doi.org") => {
                    parse_doi(nice_url)
                }
                _ => {
                    Ok(ParsedURI::HttpURL(uri.into()))
                }
            }
        }
        "arxiv" => {
            parse_arxiv(nice_url)
        }
        "doi" => {
            parse_doi(nice_url)
        }
        "akl" => {
            let name = nice_url.host_str()
                               .unwrap_or("");
            let query = nice_url.query().unwrap_or("");
            Ok(ParsedURI::AklCommand(query_to_command(name, query)?))
        }
        x => {
            log::info!("No provider attached to scheme {x}");
            anyhow::bail!("No provider attached to scheme {x}")
        }
    }
}

/// Process URI or a filepath
fn uri_or_filepath_dispatch (uri : &str) -> Result<ParsedURI> {
    match uri_dispatch (uri) {
        Ok(r) => { Ok(r) }
        Err(e) => {
            let s : String = uri.into();
            let p = PathBuf::from(s);
            if p.exists() {
                Ok(ParsedURI::FilePath(p))
            } else {
                log::error!("Error when parsing the uri {e:?}");
                log::error!("The url {uri} is neither a valid scheme nor a path on the system");
                anyhow::bail!("I don't know how to handle {uri}")
            }
        }
    }
}



/// Stupid words that should not be part of a title.
///
/// TODO: sort the words to improve binary search.
const STUPID_WORDS : &[&str] = &[
    "the", "all", "any", "one", "on", "of",
    "in", "where", "when", "why", "what",
    "this", "some", "other", "every"
];

impl Document {
    /// Document name generation.
    ///
    /// The format is
    ///    authors year title hash
    /// in lowercase and dash separated words, to simplify
    /// exploration using fzf, find or other tools.
    fn generate_name(&self) -> String {
        let authors = self.authors.iter()
            .map(|author| author.to_ascii_lowercase()
                                .replace("  ", " ")
                                .replace(' ', "-"))
            .collect::<Vec<String>>()
            .join("-");
        let year = self.year;
        let title : String = self.title
                                 .to_ascii_lowercase()
                                 .split_whitespace()
                                 .filter(|x| x.len() > 0 && !STUPID_WORDS.contains(x))
                                 .collect::<Vec<&str>>()
                                 .join("-");
        let hash = &self.checksum;
        format!("{authors} {year} {title} {hash}.pdf")
    }
}

#[derive(Serialize,Deserialize,Debug,Clone)]
struct PageArgs {
    page: Option<u32>,
    dest: Option<String>,
}
fn get_page_number(uri : &str, args : &mut CiteArgs) -> Result<()>{
    let url = Url::parse(&uri).context("Parsing URL inside document")?;
    let que = url.query().context("No query to parse")?;
    let PageArgs { page, dest } : PageArgs = serde_urlencoded::from_str(que).context("Parsing URL query")?;
    args.page = page;
    args.dest = dest;
    Ok(())
}


fn update_document_links(pdoc : &mut pdflib::PdfDocument, ident: Option<String>) {
    // TODO: allow an optional argument
    // to set a "from" path!
    // TODO forward the dest and page from
    // the link to the citation command
    pdoc.update_links(&|e| {
        let mut args = CiteArgs { uri: e.clone(),
                                  dest: None,
                                  page: None,
                                  from: ident.clone()
        };
        get_page_number(&e, &mut args).unwrap_or(());
        command_to_query(Commands::Open(args)).unwrap_or(e)
    }).unwrap();

}

fn update_document_dests(id : &str, pdoc : &mut pdflib::PdfDocument) {
    pdoc.add_destinations_links(&|e : pdflib::NamedDestination| {
        command_to_query(Commands::Cite(CiteArgs {
            uri: id.into(),
            dest: Some(e.name),
            page: Some(e.page_num),
            from: None
        })).unwrap_or("".into())
    }).unwrap();
}

fn download_pdf_document(url : &str) -> Result<pdflib::PdfDocument> {
    log::debug!("Loading document from {url}");
    let client = reqwest::blocking::Client::new();
    let mut up = Url::parse(url)?;
    up.set_query(None);
    let orig = up.to_string();
    log::debug!("Using {orig} as an origin");
    let body = client.get(url)
          .header(reqwest::header::USER_AGENT, 
                  "Rust")
          .header(reqwest::header::ACCEPT, "*/*")
          .header(reqwest::header::ACCEPT_ENCODING,
                  "Accept-Encoding: gzip, deflate, br")
          .header(reqwest::header::ACCEPT_LANGUAGE,
                  "fr,fr-FR;q=0.8,en-US;q=0.5,en;q=0.3")
          .header(reqwest::header::REFERER, &orig)
          .header(reqwest::header::CONNECTION, "keep-alive")
          .header(reqwest::header::DNT, "1")
          .header(reqwest::header::ORIGIN, &orig)
          .send()?;

    log::debug!("Pdf Document downloaded !");
    log::debug!("Status {:?}", body.status());

    let pdf = lopdf::Document::load_from(body)
        .context("parsing the pdf document in memory using lopdf")?;

    log::debug!("Pdf Document parsed !");

    let doc = pdflib::PdfDocument::try_from(pdf)
        .context("turning the parsed pdf into a fully fledged document")?;

    log::debug!("Pdf Document explored !");

    Ok(doc)
}


/// Loads a pdf document. 
/// Either from a url to download, an arxiv format,
/// or simply from a valid filepath.
fn load_pdf_document(uri : &str, identifiers : Option<&mut Vec<String>>) -> Result<pdflib::PdfDocument> {
    match uri_or_filepath_dispatch(uri)? {
        ParsedURI::FilePath(p) => {
            log::debug!("Found a direct path to import!");
            let pdf = lopdf::Document::load(p)?;
            let doc = pdflib::PdfDocument::try_from(pdf)?;
            Ok(doc)
        }
        ParsedURI::Arxiv { arxiv_id, arxiv_version } => {
            log::debug!("Found a valid arixv link to import {arxiv_id} / {arxiv_version}!");
            if let Some(ids) = identifiers {
                ids.push(format!("arxiv:{}v{}", arxiv_id, arxiv_version));
            }
            let url = format!("https://arxiv.org/pdf/{}v{}.pdf", &arxiv_id, &arxiv_version);
            download_pdf_document(&url)

        }
        ParsedURI::HttpURL(url) => {
            log::debug!("This is a direct http request");
            download_pdf_document(&url)
        }
        _ => {
            anyhow::bail!("Cannot automatically download uri {}", &uri);
        }
    }
}

/// Forward the opening of a document to the operating system.
fn forward_open(uri : &str) -> Result<()> {
    log::debug!("Opening {uri} using the system's default");
    log::debug!("Potential openers {:?}", open::commands(uri));

    open::commands(uri)[0].spawn().unwrap();
    //open::that(uri).unwrap();
    Ok(())
}

/// View a pdf file using the "best" available
/// options depending on the system.
///
/// 1. Skim / Evince / Adobe reader
/// 2. Zathura / Mupdf / Okular
/// 3. xdg-open / open / etc ...
///
/// TODO: allow this to be configured by an environment variable.
/// -> a program 
/// -> a name for the argument of destinations
/// -> a name for the argument of pages
fn view_pdf_file(path : &PathBuf, page : Option<u32>, dest: Option<String>) {
    log::info!("Opening pdf file {path:?} at {page:?} {dest:?}");
    //open::that(path).unwrap();
    let mut cmd = std::process::Command::new("evince");
    cmd.arg(path);

    if let Some(dest_name) =  dest {
        cmd.arg(format!("--named-dest={dest_name}"));
    } else if let Some(page_name) = page {
        cmd.arg(format!("--page-index={page_name}"));
    } 

    println!("args {:?}", cmd.get_args().collect::<Vec<&std::ffi::OsStr>>());

    let test = cmd.status();

    match test {
        Ok(_) => {}
        Err(_) => {
            open::commands(path)[0].spawn().unwrap();
        }
    }
}

impl AppState {
    fn new() -> Self {
        // find the correct path for the application stored state.
        // this uses ProjectDirs (cross-plateform)
        let pdirs = ProjectDirs::from("com", "aluminium", "AKL").unwrap();


        let index_path = pdirs.config_dir().join("index.yaml");
        let raw_path   = pdirs.data_dir().join("raw");
        let mod_path   = pdirs.data_dir().join("mod");
        // TODO: in modern XDG, there is XDG_STATE_DIR
        // but this is not cross platform
        let log_path   = pdirs.cache_dir().join("logs");

        // ensures that the paths exists
        // TODO: postpone this check to times we actually need
        // to open the files.
        std::fs::create_dir_all(&raw_path).unwrap();
        std::fs::create_dir_all(&mod_path).unwrap();
        std::fs::create_dir_all(&log_path).unwrap();
        std::fs::OpenOptions::new()
                .create_new(true)
                .open(&index_path).map_or((), |_| ());

        // TODO: gracefully handle failure to parse the config
        let index : Vec<Document> =
            std::fs::OpenOptions::new()
                .read(true)
                .write(false)
                .open(&index_path)
                .map(serde_yaml::from_reader)
                .unwrap()
                .unwrap();

        AppState {
            index_path,
            raw_path,
            mod_path,
            log_path,
            index,
        }
    }

    /// Delete a document from the library
    fn delete(&mut self, doc : &Document) -> Result<()> {
        let idx = self.index.iter()
                      .enumerate()
                      .find_map(|(i,d)| {
                         if d.filename == doc.filename &&
                            d.checksum == doc.checksum {
                                Some(i)
                         } else { None }
                      });
        if let Some(index) = idx {
            self.index.swap_remove(index);
        }
        Ok(())
    }


    /// Finds a document in the library.
    /// This can be quite complex, but we do the bare minimum here.
    fn find_document(&self, uri : &str) -> Result<&Document> {
        let search_result = match uri_or_filepath_dispatch(uri)? {
            ParsedURI::DOI(doi) => {
                let doi = format!("doi:{doi}");
                self.index.iter()
                          .find(|doc| {
                                    doc.identifiers.contains(&doi) })
            }
            ParsedURI::Arxiv { arxiv_version, arxiv_id } => {
                let arxiv = format!("arxiv:{arxiv_id}v{arxiv_version}");
                self.index.iter()
                          .find(|doc| {
                                    doc.identifiers.contains(&arxiv) })
            }
            ParsedURI::HttpURL(url) => {
                self.index.iter()
                          .find(|doc| {
                                    doc.identifiers.contains(&url) })
            }
            _ => {
                None
            }
        };

        match search_result {
            Some(r) => { Ok(r) }
            None    => { anyhow::bail!("Could not find {uri} in the library.") }
        }
    }

    /// Add a document to the library.
    /// Assumes that the document is valid
    /// and is not already in the library.
    fn add_document(&mut self, doc : Document, mut pdoc : pdflib::PdfDocument) -> Result<()> {
        let p = self.mod_path.join(&doc.filename);
        let r = self.raw_path.join(&doc.filename);
        pdoc.save_to(&r).context("Saving the original file to the library")?;

        update_document_links(&mut pdoc, Some(doc.identifiers[0].clone()));
        update_document_dests(&doc.identifiers[0], &mut pdoc);

        pdoc.save_to(&p).context("Saving a modified file to the library")?;

        self.index.push(doc);
        Ok(())
    }


    /// Saving the library to the yaml configuration file.
    fn save(&self) {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .read(false)
            .append(false)
            .open(&self.index_path).unwrap();
        serde_yaml::to_writer(file, &self.index).unwrap();
    }
}

fn import_document(app : &mut AppState, args : ImportArgs, interactive : bool) -> Result<String> {
    let ImportArgs { uri, authors, title, context, identifiers, year, view: _, force : _ }
    = args;
    // TODO: interactive update of the metadata using a text editor?
    // (detect if command line?)
    let mut t_identifiers = vec![];
    let mut pdf = load_pdf_document(&uri, Some(&mut t_identifiers))?;
    let met = pdf.get_meta_data()?;

    let t_authors  = if authors.len() > 0 { authors } else { met.authors };
    let t_title    = title.or(met.title).context("No title could be found")?;
    let t_checksum = pdf.get_checksum()?;
    let t_filename = "".into();

    t_identifiers.extend_from_slice(&met.identifiers);
    t_identifiers.extend_from_slice(&identifiers);
    t_identifiers.push(uri);
    t_identifiers.dedup();
    t_identifiers.sort();

    let mut t_context = vec![];
    t_context.extend_from_slice(&context);

    let t_destinations =  HashMap::new();
    let t_year = year.or(met.year).context("No year present")?;

    let mut doc = Document {
        authors: t_authors, checksum: t_checksum, filename: t_filename,
        identifiers: t_identifiers,
        title: t_title,
        year: t_year,
        context: t_context,
        destinations: t_destinations
    };

    if interactive {
        let file = tempfile::NamedTempFile::new()?;
        serde_yaml::to_writer(&file, &doc)?;
        loop {
            let proc =
                std::process::Command::new("nvim")
                    .arg(file.path())
                    .status()?;
            if proc.success() {
                break;
            }
        }
        let newfile = file.reopen()?;
        doc = serde_yaml::from_reader(&newfile).unwrap();
    }

    let name = doc.generate_name();
    doc.filename = name.clone();

    app.add_document(doc, pdf)?;
    Ok(name)
}

fn execute_command(app : &mut AppState, cmd : Commands, interactive : bool) -> Result<()> {
    log::debug!("Executing command {cmd:?} in with interactive = {interactive}");
    match cmd {
        Commands::Find => {
            app.index.iter()
                .for_each(|d| println!("{}",app.mod_path.join(&d.filename).to_string_lossy()));
        }
        Commands::Cite(CiteArgs { uri, page, dest, .. }) => {
            let mut ctx = ClipboardContext::new().unwrap();
            let citation = format!("{}?{}", 
                                   uri,
                                   serde_urlencoded::to_string(PageArgs { page, dest })?);
            ctx.set_contents(citation).unwrap();
            notifica::notify("🌍 Copied To Clipboard",
                             &format!("Copied citation of {uri}")
                            ).unwrap();
        }
        Commands::Resolve(ResolveArgs { uri }) => {
            match app.find_document(&uri) {
                Ok(doc) => {
                    println!("{:?}", &app.mod_path.join(&doc.filename));
                }
                Err(_) => {
                    println!("The document does not belong to the library");
                }
            }
        }
        Commands::Convert(ConvertArgs { uri, output }) => {
            notifica::notify("🌍 Converting",
                             &format!("Processing {}", &uri)
                            ).unwrap();
            let mut doc = load_pdf_document(&uri, None).unwrap();
            let out_path = PathBuf::from(output);
            update_document_links(&mut doc, None);
            doc.save_to(&out_path).unwrap();
            notifica::notify("🌍 Converting",
                             &format!("Finished processing {}", &uri)
                            ).unwrap();
        }
        Commands::Open(CiteArgs { uri ,page, dest, .. }) => {
            match app.find_document(&uri) {
                Ok(doc) => {
                    log::debug!("Document {uri} already exists");
                    view_pdf_file(&app.mod_path.join(&doc.filename), page, dest);
                }
                Err(_) => {
                    log::debug!("Document {uri} was not found");
                    forward_open(&uri)?;
                }
            }
        }
        Commands::View(CiteArgs { uri, page, dest,.. }) => {
            view_pdf_file(&PathBuf::from(uri), page, dest);
        }
        Commands::Import(import_args) => {
            notifica::notify("🌍 Converting",
                             &format!("Processing {}", import_args.uri)
                            )
                .context("Notifying the user that the conversion started")?;
            log::info!("Importing document {}", import_args.uri);
            let m_doc = app.find_document(&import_args.uri);
            let view = import_args.view;
            let name : String;

            match (m_doc, import_args.force) {
                (Ok(doc), false) => {
                    log::info!("Document {} already in the library, but force set to false", import_args.uri);
                    name = doc.filename.clone();
                }
                (Ok(doc), true)  => {
                    log::info!("Document {} already in the library, and force set to true", import_args.uri);
                    app.delete(&doc.clone())?;
                    name = import_document(app, import_args, interactive)?;
                }
                (Err(_), _)    => {
                    log::info!("Document {} is completely new", import_args.uri);
                    name = import_document(app, import_args, interactive)?;
                }
            };

            notifica::notify("🌍 Converting",
                             &format!("Finished processing {name}")
                            )
                .context("Notifying the user that the conversion is done")?;


            if view {
                view_pdf_file(&app.mod_path.join(name), None, None)
            }

        }
    }
    app.save();
    Ok(())
}

fn main() {
    let mut app = AppState::new();

    let log = file_rotate::FileRotate::new(
        app.log_path.join("akl-rs"),
        file_rotate::suffix::AppendCount::new(2),
        file_rotate::ContentLimit::Lines(1000),
        file_rotate::compression::Compression::None,
        #[cfg(unix)]
        None,
    );

    let mut log_builder = env_logger::Builder::from_default_env();
    log_builder
        .target(env_logger::Target::Pipe(Box::new(log)))
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::debug!("Parsing CLI");
    //log::debug!("Current app state is {app:?}");

    let cli = Cli::parse();

    match cli.execute_uri {
        Some(val) => {
            log::info!("Custom uri found {val:?}, will parse it.");
            match uri_or_filepath_dispatch(&val) {
                Ok(ParsedURI::DOI(doi)) => {
                    println!("Please add a verb to this doi: {doi}");
                }
                Ok(ParsedURI::Arxiv { arxiv_id, arxiv_version }) => {
                    println!("Please add a verb to this arxiv identifier: {arxiv_id} {arxiv_version}");
                }
                Ok(ParsedURI::HttpURL(url)) => {
                    println!("Please add a verb to this http url: {url}");
                }
                Ok(ParsedURI::FilePath(path)) => {
                    println!("Please add a verb to this filepath: {path:?}");
                }
                Ok(ParsedURI::AklCommand(cmd)) => {
                    execute_command(&mut app, cmd, cli.interactive).unwrap()
                }
                Err(e) => {
                    log::error!("Could not parse the argument {e:?}");
                    println!("Invalid argument");
                }
            }
        }
        None => {
            log::info!("Regular command mode");
            match cli.command {
                Some(cmd) => { execute_command(&mut app, cmd, cli.interactive).unwrap() }
                None => { println!("Please execute something") } 
            }
        }
    }
}
