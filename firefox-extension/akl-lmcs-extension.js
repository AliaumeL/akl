setTimeout(() => {
  "use strict";

  const meta = (field) =>
    document.querySelector(`meta[name="${field}"]`)?.content;

  const DOI = meta("citation_doi") || meta("DC.identifier");
  const publisher = meta("citation_journal_title");

  const date = meta("citation_date");
  const year = parseInt(date.substring(0, 4));

  const citation_url = meta("citation_pdf_url");
  const title = meta("citation_title");

  const authors = Array.from(
    document.querySelectorAll("meta[name='citation_author']")
  ).map((x) => x.content);

  const identifiers = [
    `https://dx.doi.org/${DOI}`,
    `doi:${DOI}`,
    window.location.href,
  ];

  const context = [];
  if (publisher) {
    context.push(publisher);
  }

  const query = new URLSearchParams({
    payload: JSON.stringify({
      uri: citation_url,
      title,
      year,
      authors,
      identifiers,
      context,
      view: true,
      force: false,
    }),
  });

  const url = "akl://import-document/?" + query.toString();

  const panel = document.querySelector(".panel-body");
  const link = document.createElement("a");
  link.href = url;
  panel.append(link);
  link.marginLeft = "5px";
  const btn = document.createElement("button");
  link.append(btn);
  btn.className = "btn btn-default btn-sm";
  btn.style.marginRight = "5px";
  const spn = document.createElement("span");
  spn.className = "fas fa-file-download";
  spn.style.marginRight = "5px";
  btn.innerText = "AKL IMPORT";
  btn.prepend(spn);
});
