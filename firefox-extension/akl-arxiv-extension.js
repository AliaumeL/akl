(function () {
  "use strict";

  const meta = (field) =>
    document.querySelector(`meta[name="${field}"]`).content;

  const download = document.querySelector(".download-pdf").href;
  const arxivID = download.substring(22);

  const identifiers = [
    `arXiv:${arxivID}`,
    download,
    `https://arxiv.org/abs/${arxivID}`,
  ];

  const title = meta("citation_title");
  const date = meta("citation_online_date");

  const authors = Array.from(
    document.querySelectorAll("meta[name='citation_author']")
  ).map((x) => x.content);

  const year = date.substring(0, 4);
  //const abstr = meta("citation_abstract");

  const storage = "pdf-storage/";

  const query = new URLSearchParams({
    download,
    storage,
    document: JSON.stringify({
      checksum: "",
      filename: `${authors} - ${title} - ${year} - ${arxivID}`,
      title,
      year,
      authors,
      identifiers,
      //abstract: abstr,
    }),
  });

  const url = "akl://import-document/?" + query.toString();
  const list = document.querySelector(".extra-services .full-text ul");
  const li = document.createElement("li");
  const a = document.createElement("a");
  //a.target = "_blank";
  a.innerHTML = "AKL IMPORT";
  a.href = url;
  li.appendChild(a);
  list.appendChild(li);
})();
