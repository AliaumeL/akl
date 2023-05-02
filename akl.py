#!/usr/bin/env python3

# Logging

import logging

# Python typing and functional programming
from pydantic import BaseModel, validator, Field
from typing import List, Union, Optional, Dict, Any, Callable
import itertools

# Date and time...
import datetime

# Path,system and os
import os
import platform
import subprocess
from pathlib import Path
import tempfile
import shutil

# PDF treatment libs
import pypdf
from pypdf.generic import AnnotationBuilder

# Document parsing/extracting
import re
import json
import yaml
import hashlib
# from bs4 import BeautifulSoup
# import magic
# import feedparser

# Web interactions
# import requests
import httpx
import requests
from urllib.parse import urlparse, urljoin, parse_qs, urlencode

# GUI interactions
import clipboard

# TUI interactions
import click

default_dir = "/home/alopez/Code/akl/pdf-storage"

### LOGGING
logging.basicConfig(filename="/home/alopez/Code/akl/akl.log",
                    filemode='a',
                    format='%(asctime)s,%(msecs)d %(name)s %(levelname)s %(message)s',
                    datefmt='%H:%M:%S',
                    level=logging.DEBUG)

### CORE UTILITY FUNCTIONS ###

#
# For older python versions,
# we need to write our own way
# to build sha256sums of files
#
def sha256sum(filename: Path) -> str:
    """
    Computes the sha256sum of the
    file at the given path.

    Warning: does not check that the file exists.
    Warning: opens the file in read only binary mode.
    """
    h = hashlib.sha256()
    b = bytearray(128 * 1024)
    mv = memoryview(b)
    with open(filename, "rb", buffering=0) as f:
        for n in iter(lambda: f.readinto(mv), 0):
            h.update(mv[:n])
    return h.hexdigest()

def maybe_of_list(l: List[Any]) -> Optional[Any]:
    if len(l) == 1:
        return l[0]
    else:
        return None

def non_empty_intersection(l1 : List[Any], l2 : List[Any]) -> bool:
    return len(set(l1).intersection(l2)) > 0

class Destination(BaseModel):
    """A wrapper class for type safe
       pdf destination handling.
    """

    left: float
    top: float
    page: int
    names: List[str]


def destinations_from_pdf(reader: pypdf.PdfReader) -> List[Destination]:
    """Extract the list of pdf named destinations
    from a given pdf reader
    """
    # Get all the destinations
    infos = reader.named_destinations
    # Regroup the destinations based
    # on their location in the pdf file
    # (left,right,page_number)
    locations = {}
    for info in infos.values():
        if info.title.strip() != "" and info.left is not None and info.top is not None:
            p = reader.get_destination_page_number(info)
            k = (info.left, info.top, p)
            locations.setdefault(k, []).append(info)
    return [
        Destination(left=left, top=top, page=page, names=[dest.title for dest in dests])
        for ((left, top, page), dests) in locations.items()
        if isinstance(left,float) and isinstance(top,float) and
        isinstance(page,int)
    ]

def custom_pdf_creation(
    reader: pypdf.PdfReader,
    linker: Callable[[Destination], List[pypdf.generic.DictionaryObject]],
    rewriter: Callable[[str], str]
) -> pypdf.PdfWriter:
    """ Takes a pdf as input, produces a customised pdf as output.

        for each external link inside the pdf, the url is rewritten
        using the `rewriter` function given in argument.

        for each named destination inside the pdf, the
        `linker` function is called to add a corresponding annotation
        on the corresponding page.
    """
    writer = pypdf.PdfWriter()
    writer.clone_document_from_reader(reader)

    for page in writer.pages:
        if "/Annots" in page:
            for annot in page["/Annots"].get_object():
                annot_obj = annot.get_object()
                slash_a = annot_obj.get("/A", None)
                if slash_a is not None:
                    uri = slash_a.get_object().get("/URI", None)
                else:
                    uri = None
                if uri is not None:
                    annot_attr = annot_obj["/A"]
                    new_uri = pypdf.generic.TextStringObject(rewriter(uri))
                    new_key = pypdf.generic.NameObject("/URI")
                    annot_attr.update({new_key: new_uri})

    for dest in destinations_from_pdf(reader):
        for obj in linker(dest):
            writer.add_annotation(page_number=dest.page, annotation=obj)

    return writer

Identifier = str

def identifier_of_uri(uri : str) -> Identifier:
    """ smart parsing of identifiers """
    parsed = urlparse(uri)
    if parsed.scheme == "https" or parsed.scheme == "http":
        if parsed.netloc == "arxiv.org":
            return f"arXiv:{parsed.path[5:]}"
        elif parsed.netloc == "doi.org" or parsed.netloc == "dx.doi.org":
            return f"doi:{parsed.path[1:]}"
        else:
            return uri
    else:
        return uri

class PdfMetaData(BaseModel):
    """
    This is the metadata that we associate
    to a given pdf file.
    """
    checksum: str
    identifiers: List[Identifier]
    filename: str
    # Optional Metadata 
    # To Further Identify
    # The Document
    title: Optional[str] = None
    authors: List[str] = []
    year: Optional[str] = None
    publisher: Optional[str] = None 

def generate_name(m: PdfMetaData) -> str:
    """Takes metadata as input
    and produces a not-so-stupid
    name based on the idea
    of
    [AUTHORS][YEAR][TITLE][WHERE][UID].pdf
    WHERE = arxiv / LICS / etc ...
    """
    assert m.checksum != ""
    authors = "".join(author[:2] for author in m.authors).upper()
    year = m.year or "____"
    title = (
        "".join(
            itertools.islice(
                (
                    word
                    for word in map(lambda x: x.lower(), (m.title or "").split(" "))
                    if word
                    not in ["the", "all", "any", "a", "one", "on", "of", "in", "where", "when"]
                ),
                1,
            )
        )
        or "untitled"
    ).replace("-", "")
    where = m.publisher or "L O C A L"
    where = "".join(
        word[0]
        for word in where.split(" ")
        if word not in ["Annual", "Proceedings", "Symposium"]
        if len(word) > 2 and word[0].isupper() and word[1].islower()
    )

    return " ".join(x for x in [authors, year, title, where, m.checksum] if x != "")

class PdfLibrary(BaseModel):
    """
    The dumbest way to handle a PDF library.
    storage is a directory that contains
        - index.yaml
        - storage/backup/*.pdf (backup versions of the pdfs)
        - storage/edit/*.pdf (edit versions of the pdfs)
    """
    storage: Path
    # For now, the database is simply
    # a list of available papers and their statuses
    data: List[PdfMetaData]

    @staticmethod
    def load_from(p: Path) -> "PdfLibrary":
        index = p / "index.yaml"
        edit = p / "edit"
        raw = p / "raw"
        cache = p / "_cache"

        index.touch(exist_ok = True)
        edit.mkdir (exist_ok = True)
        cache.mkdir(exist_ok = True)
        raw.mkdir  (exist_ok = True)

        with open(index, "r") as stream:
            return PdfLibrary(storage=p, data=yaml.safe_load(stream) or [])

    def find_similar_to(self, m : PdfMetaData) -> Optional[PdfMetaData]:
        l = [  d for d in self.data if
                 d.checksum == m.checksum
                 or
                 non_empty_intersection(m.identifiers, d.identifiers) ]
        return maybe_of_list(l)

    def save(self):
        with open(self.storage / "index.yaml", "w") as stream:
            yaml.dump([d.dict(exclude_none=True) for d in self.data], stream)

    def find(self, uri: str) -> Optional[PdfMetaData]:
        """ Find pdfs that could be connected to a given uri """
        for doc in self.data:
            if uri in doc.urls or uri in doc.uids or uri == doc.filename:
                return doc
        return None

    def duplicates(self, checksum: str) -> List[PdfMetaData]:
        """ find all pdfs that have a collision with a given checksum """
        return [doc for doc in self.data if doc.checksum == checksum]

    def list_duplicates(self) -> Dict[str,List[PdfMetaData]]:
        """ Helper function to find all clashes of pdfs hashes """
        res = {}
        for m in self.data:
            res.setdefault(m.checksum, []).append(m)
        return res

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.save()


#### THE ACTUAL IMPLEMENTATION OF THE PROTOCOL
#
# 1) open a given file (maybe it is NOT in the library!!)
# ----> if it is a filepath, then open it (and tmp build)
# ----> else, if it is in the library, then open it
# ----> else, if it is a known url scheme, import and open
# ----> otherwise, then default_open
# 2) import a given uri in the library
# ----> TODO: this is the hard part
# 3) cite
#

class OpenArgs(BaseModel):
    uri: str
    storage: Path
    dest: Optional[str] = None
    page: Optional[int] = None
    bibtex: Optional[Path] = None
    knowledge: Optional[Path] = None

class ImportArgs(BaseModel):
    download: str
    document: PdfMetaData
    storage: Path

class CiteArgs(BaseModel):
    uri: str
    storage: Path
    dest: str
    page: int
    bibtex: Optional[Path] = None
    knowledge: Optional[Path] = None

AppArgs = Union[OpenArgs, ImportArgs, CiteArgs]

def args_of_uri(uri: str) -> Optional[AppArgs]:
    parsed = urlparse(uri)
    logging.debug(f"[parsing] : {parsed}")
    query = parse_qs(parsed.query)

    if parsed.scheme != "akl":
        raise ValueError(f"Unknown protocol {parsed.scheme}")

    query = {k: v[0] for (k, v) in query.items()
                     if len(v) > 0 and v[0] is not None}
    logging.debug(f"[parsing] : {query}")

    if parsed.netloc == "open-document":
        return OpenArgs.parse_obj(query)
    elif parsed.netloc == "cite":
        return CiteArgs.parse_obj(query)
    elif parsed.netloc == "import-document":
        logging.debug(f"[parsing] : this is import-document link!")
        try:
            query["document"] = PdfMetaData.parse_raw(query["document"])
        except Exception as e:
            logging.error(f"Unable to parse the document: {e}")
        logging.debug(f"[parsing] : {query}")
        return ImportArgs.parse_obj(query)
    else:
        raise ValueError(f"Unknown command {parsed.netloc}")


def uri_of_args(args: AppArgs) -> str:
    if isinstance(args, CiteArgs):
        scheme = "akl://cite/?"
    elif isinstance(args, OpenArgs):
        scheme = "akl://open-document/?"
    elif isinstance(args, OpenArgs):
        scheme = "akl://import-document/?"
    else:
        raise ValueError(f"Invalid args {args}")

    params = args.dict(exclude_none=True)
    return f"{scheme}{urlencode(params)}"

### WRAPPER FUNCTIONS THAT DO ONE THING ###

def open_linux(path):
    subprocess.call(["xdg-open", path])

def open_osx(path):
    subprocess.call(["open", path])

def open_win(path):
    os.startfile(path)

def open_pdf_evince(path : Path, dest : Optional[str], page : Optional[int]):
    if dest is not None:
        return subprocess.run(["evince", path, f"--named-dest={page}"])
    elif page is not None:
        return subprocess.run(["evince", path, f"--page-label={page}"])
    else:
        return subprocess.run(["evince", path])

def open_pdf_zathura(path : Path, dest : Optional[str], page : Optional[int]):
    if page is not None:
        return subprocess.run(["zathura", path, f"--page={page}"])
    else:
        return subprocess.run(["zathura", path])

def open_pdf_skim(path : Path, dest : Optional[str], page : Optional[int]):
    return subprocess.run(["skim", path])

def open_pdf_acrobat(path : Path, dest : Optional[str], page : Optional[int]):
    return subprocess.run(["AcroRd32.exe", path])

if platform.system() == 'Darwin':       # macOS
    open_default = open_osx
    open_pdf_at = open_pdf_skim
elif platform.system() == 'Windows':    # Windows
    open_default = open_win
    open_pdf_at = open_pdf_acrobat
else:                                   # linux variants
    open_default = open_linux
    open_pdf_at = open_pdf_zathura

#### 

def create_edited_pdf(
        args : OpenArgs,
        rawpath : Path,
        expath: Path):
    """ The actual implementation that creates the custom pdf """
    logging.debug(f"Starting to read {rawpath} to produce {expath}")
    reader = pypdf.PdfReader(rawpath)

    def linker(d: Destination) -> List[pypdf.generic.DictionaryObject]:
        # rect [xll,yll, xur,yur]
        rect = [d.left - 2, d.top - 2, d.left + 2, d.top + 2]
        return [
            AnnotationBuilder.free_text(
                "A",
                rect=rect,
                font="Arial",
                font_size="10pt",
                background_color="8FBCBB",
                border_color="8FBCBB",
            ),
            AnnotationBuilder.link(
                rect=rect,
                url=uri_of_args(
                    CiteArgs(
                        uri=args.uri,
                        storage=args.storage,
                        bibtex=args.bibtex,
                        knowledge=args.knowledge,
                        dest=d.names[0],
                        page=d.page,
                    )
                ),
            ),
        ]

    def rewriter(link: str) -> str:
        """ Create a akl link wrapper
            note that if the link contains
            arguments such as "page"
            or "dest", then those are
            lifted to the wrapper too!
        """
        parsed = urlparse(link)
        query = parse_qs(parsed.query)
        return uri_of_args(
            OpenArgs(
                uri=link,
                storage=args.storage,
                page=maybe_of_list(query.get("page", [])),
                dest=maybe_of_list(query.get("dest", [])),
            )
        )

    writer = custom_pdf_creation(reader, linker, rewriter)
    with open(expath, "wb") as fp:
        writer.write(fp)
    logging.debug(f"Finished writing to {expath}")


## The lib resolver

def maybe_resolve_filepath(p : Path, lib : PdfLibrary) -> Optional[PdfMetaData]:
    if not p.exists():
        return None
    checksum = sha256sum(p)
    dups = lib.duplicates(checksum)
    if len(dups) == 1:
        return dups[0]

def maybe_resolve_identifier(i : Identifier, lib: PdfLibrary) -> Optional[PdfMetaData]:
    return maybe_of_list([
        m for m in lib.data 
        if i in m.identifiers
    ])

def maybe_resolve_uri(uri : str, lib : PdfLibrary) -> Optional[PdfMetaData]:
    # (1) check if it is a path
    p = Path(uri)
    m1 = maybe_resolve_filepath(p, lib)
    if m1 is not None:
        return m1
    # (2) extract/cleanup an identifier (possibly)
    uid = identifier_of_uri(uri)
    logging.debug(f"Normalising the uri to {uid}")
    m2 = maybe_resolve_identifier(uid, lib)
    if m2 is not None:
        return m2
    # (3) otherwise... we probably do not have the document
    return None

###

def do_open(args: OpenArgs):
    lib = PdfLibrary.load_from(args.storage)
    with lib:
        m = maybe_resolve_uri(args.uri, lib)

    # ----> if it is in the library, then open it
    if m is not None:
        logging.debug(f"{args.uri} was already in the library")
        rawpath = args.storage / "raw" / m.filename
        expath  = args.storage / "edit" / m.filename
        if not expath.exists():
            logging.debug(f"{args.uri} was not computed before")
            create_edited_pdf(args, rawpath, expath)
        logging.debug(f"open {expath} at {args.dest} page {args.page}")
        return open_pdf_at(expath, args.dest, args.page)
    # ----> if it is a filepath, then open it (and tmp build)
    true_path = Path(args.uri)
    if true_path.exists():
        logging.debug(f"{args.uri} was a path")
        # The file exists
        checksum = sha256sum(true_path)
        dups = lib.duplicates(checksum)
        expath = lib.storage / "_cache" / f"{checksum}.pdf"
        if not expath.exists():
            logging.warning(f"{args.uri} was not computed before")
            create_edited_pdf(args, true_path, expath)
        else:
            logging.warning(f"{args.uri} was in the cache")
        return open_pdf_at(expath, args.dest, args.page)
    logging.warning(f"{args.uri} was not in the library")
    # ----> otherwise, default_open
    return open_default(args.uri)

def do_import(args: ImportArgs) -> Optional[PdfMetaData]:
    # TODO:
    # --> what to do if it is already on the system?
    # the download link is enough to download the document,
    # but the extra metadata is used to handle duplicate
    # files (and potentially not download)
    lib = PdfLibrary.load_from(args.storage)
    m = lib.find_similar_to(args.document)
    if m is not None:
        logging.warning(f"Importing a document that was already present {args.document}")
        with lib:
            m.identifiers = list(set([*args.document.identifiers,*m.identifiers]))
        do_open(OpenArgs(uri=m.identifiers[0], storage=args.storage))
        return m

    # There are no similar documents
    # in the library, so download
    path = Path(args.download)
    if path.exists():
        logging.info(f"The {args.download} is probably a local file")
        with open(path, "rb") as f:
            content = f.read()
    else:
        logging.info(f"The {args.download} is probably an url")
        parsed = urlparse(args.download)
        newurl = parsed._replace(query=None).geturl()
        logging.info(f"The base url is {newurl}")
        headers = {
                'User-Agent': 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/110.0',
                'Accept': '*/*',
                'Accept-Language': 'fr,fr-FR;q=0.8,en-US;q=0.5,en;q=0.3',
                'Accept-Encoding': 'gzip, deflate, br',
                'referer': newurl,
                'origin': newurl,
                'DNT': '1',
                'Connection': 'keep-alive',
                'Sec-Fetch-Dest': 'empty',
                'Sec-Fetch-Mode': 'cors',
                'Sec-Fetch-Site': 'cross-site',
            }

        if parsed.netloc == "arxiv.org":
            answ = requests.get(args.download, headers=headers)
        else:
            answ = httpx.get(args.download, headers=headers)

        if answ.status_code != 200:
            logging.error(f"unable to fetch {args.document} using {args.newurl}")
            return None

        content = answ.content

    # Compute the actual checksum
    checksum = hashlib.sha256(content).hexdigest()

    # let's see again if we did not already had
    # this file ^^
    dups = lib.duplicates(checksum)
    if len(dups) == 1:
        logging.info(f"The document {args.document} has a checksum clash")
        with lib:
            m = dups[0]
            dups[0].identifiers = list(set([*args.document.identifiers,*m.identifiers, args.download]))
            return dups[0]

    # TODO: check the filetype using `magic`
    args.document.checksum = checksum
    name = generate_name(args.document)
    args.document.filename = name

    logging.info(f"Writing {name} for the current document")

    with open(lib.storage / "raw" / name, "wb") as f:
        f.write(content)
        f.flush()
    with lib:
        lib.data.append(args.document)
        logging.info(f"{checksum} added to the database")

    if len(args.document.identifiers) > 0:
        do_open(OpenArgs(uri=args.document.identifiers[0], storage=args.storage))
    else:
        do_open(OpenArgs(uri=args.download, storage=args.storage))
    return args.document

def do_cite(args: CiteArgs):
    # TODO:
    # lib = PdfLibrary.load_from(args.storage)
    # with lib:
        # m = maybe_resolve_uri(args.uri, lib)
    # if m is not None:
        #TODO: i = find_good_identifier(m)
    i = args.uri
    cmd = f"\\url{{{i}?page={args.page}&dest={args.dest}}}"
    clipboard.copy(cmd)
    # else:
        # logging.warning(f"{args.uri} does not exist...")

def do_command(args: AppArgs):
    if isinstance(args, CiteArgs):
        return do_cite(args)
    elif isinstance(args, OpenArgs):
        return do_open(args)
    elif isinstance(args, ImportArgs):
        return do_import(args)
    else:
        raise ValueError("unknown command")

#### COMMAND LINE INTERFACE

@click.group("akl")
def akl():
    pass

@akl.command()
@click.option(
    "--storage",
    default=default_dir,
    type=click.Path(exists=True, file_okay=False, dir_okay=True),
)
@click.option("--title", default = None)
@click.option("--authors", multiple=True, default = [])
@click.option("--publisher", default=None)
@click.option("--identifiers", multiple=True, default =[])
@click.option("--year", default=None)
@click.argument("download")
def import_document(download : str,
                    storage : str,
                    identifiers : List[str],
                    title : Optional[str],
                    authors : List[str],
                    publisher : Optional[str],
                    year : Optional[str]):
    """ Opens in exploratory mode a given file """
    return do_import(
        ImportArgs(
            storage=Path(storage).resolve(),
            download=download,
            document=PdfMetaData(checksum="",
                                 filename=download,
                                 title=title,
                                 identifiers=[*identifiers,download],
                                 authors=authors,
                                 publisher=publisher,
                                 year=year)
        )
    )

@akl.command()
@click.option(
    "--storage",
    default=default_dir,
    type=click.Path(exists=True, file_okay=False, dir_okay=True),
)
@click.option(
    "--page",
    default=None,
    type=click.INT,
)
@click.option(
    "--dest",
    default=None,
    type=click.STRING
)
@click.argument("document")
def open_document(document, storage,page,dest):
    """ Opens in exploratory mode a given file """
    return do_open(
        OpenArgs(
            uri=document,
            storage=Path(storage).resolve(),
            page=page,
            dest=dest
        )
    )


@akl.command()
@click.option(
    "--storage",
    default=default_dir,
    type=click.Path(exists=True, file_okay=False, dir_okay=True),
)
@click.argument("document")
def resolve_document(document,storage):
    """ Gives the filepath to the given document """
    pass # TODO: implement

@akl.command()
@click.argument("akl-uri")
def follow(akl_uri):
    """ Follows the uri given for akl links """
    logging.debug(f"Following link {akl_uri}")
    command = args_of_uri(akl_uri)
    logging.debug(f"Link parsed: {command}")
    if command is not None:
        do_command(command)


if __name__ == "__main__":
    akl()
