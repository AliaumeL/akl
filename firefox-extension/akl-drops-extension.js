setTimeout(() => {
  "use strict";

  const meta = (field) =>
    document.querySelector(`meta[name="${field}"]`)?.content;

  const DOI = meta("citation_doi") || meta("DC.identifier");
  const publisher =
    meta("citation_journal_title") || meta("citation_conference_title");

  const date = meta("citation_date");
  const year = parseInt(date.substring(0, 4));

  const citation_url = meta("citation_pdf_url");
  const title = meta("citation_title");

  const authors = Array.from(
    document.querySelectorAll("meta[name='citation_author']")
  ).map((x) => x.content);

  const identifiers = [
    `https://dx.doi.org/${DOI}`,
    window.location.href,
    `doi:${DOI}`,
  ];
  const context = [];
  if (publisher) {
    context.push(publisher);
  }

  const payload = {
    uri: citation_url,
    title,
    year,
    authors,
    identifiers,
    context,
    view: true,
    force: false,
  };

  console.log(payload);

  const query = new URLSearchParams({
    payload: JSON.stringify(payload),
  });

  const url = "akl://import-document/?" + query.toString();

  const table = document.querySelectorAll("tbody")[0];

  const link = document.createElement("a");
  const tr = document.createElement("tr");
  const dlTitle = document.createElement("td");
  const dlUrl = document.createElement("td");

  link.innerHTML = "IMPORT DOCUMENT";
  link.href = url;
  dlTitle.innerHTML = "akl import:";
  dlUrl.append(link);
  tr.appendChild(dlTitle);
  tr.appendChild(dlUrl);
  table.appendChild(tr);
}, 10);
