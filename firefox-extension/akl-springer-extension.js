(function () {
  "use strict";

  const meta = (field) =>
    document.querySelector(`meta[name="${field}"]`)?.content;

  const DOI = meta("citation_doi") || meta("DOI");
  const conference = meta("citation_conference_abbrev");
  const download = meta("citation_pdf_url");

  const identifiers = [`https://dx.doi.org/${DOI}`, `doi:${DOI}`, download];

  const title = meta("citation_title");
  const date = meta("citation_publication_date");

  const authors = Array.from(
    document.querySelectorAll("meta[name='citation_author']")
  ).map((x) => x.content);

  const year = parseInt(date.substring(0, 4));
  const context = [];
  //const abstr = meta("citation_abstract");

  const query = new URLSearchParams({
    payload: JSON.stringify({
      uri: download,
      title,
      authors,
      context,
      identifiers,
      view: true,
      force: false,
    }),
  });

  const url = "akl://import-document/?" + query.toString();
  const list = document.querySelector("div.c-pdf-container");
  const div = document.createElement("div");
  const a = document.createElement("a");
  //a.target = "_blank";
  a.innerHTML = "AKL IMPORT";
  a.href = url;
  a.className =
    "u-button u-button--full-width u-button--primary u-justify-content-space-between c-pdf-download__link";
  div.appendChild(a);
  div.className = "c-pdf-download";
  list.appendChild(div);

  const aside = document.querySelector("aside div.c-pdf-container");
  aside.appendChild(div);
})();
