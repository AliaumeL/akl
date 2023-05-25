#!/usr/bin/env python3
import sys
import pathlib
import bibtexparser
import subprocess
from typing import List, Dict, Optional


def find_citations(tex_root : pathlib.Path) -> List[Dict]:
    citations = []
    for bib_file in tex_root.glob('**/*.bib'):
        with open(bib_file,"r") as bibtex_file:
            bib_database = bibtexparser.load(bibtex_file)
            citations.extend(bib_database.entries)
    return citations


def select_citation(citations : List[Dict]) -> Optional[str]:
    dmenu_input = []
    for citation in citations:
        ID  = citation.get("ID", None)
        title   = citation.get("title", citation.get("booktitle", "-"))
        authors = citation.get("author", "-")
        year    = citation.get("year", "-")
        title = title[:30]
        dmenu_input.append(f"{ID}\t{year}\t{title}\t{authors}")
    options = '\n'.join(option.replace('\n', ' ') for option in dmenu_input)
    cmd     = ["rofi",
               '-dmenu', '-p', "Select Citation", '-format', 's', '-i',
               '-lines', '5']
    result = subprocess.run(cmd, input=options, 
                    stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL,
                    universal_newlines=True)

    if result.returncode == 0:
        stdout = result.stdout.strip()
        return stdout[:stdout.index("\t")]
    else:
        return None


def open_citation():
    tex_root  = pathlib.Path(sys.argv[2])
    citations = find_citations(tex_root)

    filtered_citations_url = {}
    filtered_citations = []
    for citation in citations:
        ID  = citation.get("ID", None)
        DOI = citation.get("DOI", None) or citation.get("doi",None)
        URL = citation.get("url", None) or citation.get("URL",None)
        link = URL or (DOI and f"https://dx.doi.org/{DOI}")
        if ID and link:
            filtered_citations_url[ID] = link
            filtered_citations.append(citation)

    citation_id = select_citation(filtered_citations)
    if citation_id is not None:
        selected_url = filtered_citations_url[citation_id]
        print(citation_id,end="")
        subprocess.Popen(["akl", "open", "--uri", selected_url],
                    stderr=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL)
    else:
        print("Error")

def create_citation():
    ID   = sys.argv[2]
    name = sys.argv[3]
    url  = sys.argv[4]
    print("\\akldef{")
    print(f"\tkey={ID},")
    print(f"name={name},")
    print(f"url={url}")
    print("}")

def insert_citation():
    tex_root  = pathlib.Path(sys.argv[2])
    citations = find_citations(tex_root)

    citation_id = select_citation(citations)
    if citation_id is not None:
        print(citation_id.strip(),end="")


if __name__ == '__main__':
    mode = sys.argv[1]

    if mode == "open":
        open_citation()
    elif mode == "create":
        create_citation()
    elif mode == "insert":
        insert_citation()
    else:
        exit(1)
