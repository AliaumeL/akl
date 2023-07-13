#!/usr/bin/env python3
import sys
import re
import pathlib
import bibtexparser
import subprocess
import clipboard
from urllib.parse import urlparse, parse_qs
from typing import List, Dict, Optional, Tuple

def find_citations(tex_root : pathlib.Path) -> List[Dict]:
    citations = []
    for bib_file in tex_root.glob('**/*.bib'):
        with open(bib_file,"r") as bibtex_file:
            bib_database = bibtexparser.load(bibtex_file)
            citations.extend(bib_database.entries)
    return citations

def open_citation():
    tex_root  = pathlib.Path(sys.argv[2])
    bibkey    = pathlib.Path(sys.argv[3])
    citations = find_citations(tex_root)

    for citation in citations:
        if citation["ID"] == bibkey:
            DOI = citation.get("DOI", None) or citation.get("doi",None)
            URL = citation.get("url", None) or citation.get("URL",None)
            arxiv = citation.get("eprint", None)
            link = (URL or
                    (DOI and f"https://dx.doi.org/{DOI}") or
                    (arxiv and f"arxiv:{arxiv}"))
            if link is not None:
                subprocess.Popen(["akl", "open", "--uri", link],
                            stderr=subprocess.DEVNULL,
                            stdout=subprocess.DEVNULL)
            return None

def citation_identities(citation: Dict) -> List[str]:
    return [ citation[x] for x in ["DOI", "doi", "url", "URL", "eprint", "arxiv"]
            if x in citation ]

def citation_matches(citation: Dict, path : str) -> bool:
    return any((x in path) or (path in x)
            for x in citation_identities(citation))


def create_kl_citation(citation: Dict, url : str, dest : Optional[str], page: Optional[str]) -> str:
    ID = citation["ID"]
    name = dest if dest is not None else page
    return f"""
\\knowledge{{url={{{url}}},bibkey={{{ID}}}}}
  | {name}@{ID}"""


def parse_kl_citations(file : pathlib.Path) -> Dict:
    regdef = r"\\knowledge{url={(.+)},\s*bibkey={(.+)}}"
    regsta = r"\s+\|\s+([^@]+)"
    result = {}
    current_key = None
    current_url = None
    with open(file,"r") as f:
        for line in f.readlines():
            x = re.search(regdef, line)
            y = re.match(regsta, line)
            if x is not None:
                current_url = x.group(1).strip()
                current_key = x.group(2).strip()
            elif y is not None:
                result[current_url] = {
                        "key": current_key,
                        "tag": y.group(1).strip()
                }
                current_key = None
                current_url = None
            else:
                current_key = None
                current_url = None
    return result



def create_citation():
    tex_root  = pathlib.Path(sys.argv[2])
    if len(sys.argv) >= 4:
        kl_file   = pathlib.Path(sys.argv[3])
    else:
        kl_file = tex_root / "knowledges.tex"
        kl_file.touch(exist_ok=True)

    pasted_url = clipboard.paste()

    quotes = parse_kl_citations(kl_file)
    if pasted_url in quotes:
        quote = quotes[pasted_url]
        print("\\cite["+quote["tag"]+"]{"+quote["key"]+"}",end="")
    else:
        citations = find_citations(tex_root)
        parsed = urlparse(pasted_url)
        query   = parse_qs(parsed.query)

        dest = query.get("dest",[])
        dest = dest[0] if len(dest) > 0 else None

        page = query.get("page", [])
        page = page[0] if len(page) > 0 else None

        comment = dest or page

        candidates = [citation for citation in citations
                      if citation_matches(citation, parsed.path) ]
        if len(candidates) == 0:
            print("NOT FOUND")
        elif len(candidates) == 1:
            if comment is not None:
                print("\\cite[" + comment + "]{" + candidates[0]["ID"] + "}",end ="")
            else:
                print("\\cite{" + candidates[0]["ID"] + "}", end="")

            with open(kl_file, "a") as f:
                f.write(create_kl_citation(candidates[0], pasted_url, dest,
                    page))
        else:
            print("TOO MANY CANDIDATES")


if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == "open":
        open_citation()
    elif mode == "create":
        create_citation()
    else:
        exit(1)
