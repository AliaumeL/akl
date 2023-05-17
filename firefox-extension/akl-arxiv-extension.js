(function () {
  "use strict";

  const meta = (field) =>
    document.querySelector(`meta[name="${field}"]`).content;

  const download = document.querySelector(".download-pdf").href;
  const arxivID = download.substring(22);

  const identifiers = [
    `https://arxiv.org/abs/${arxivID}`,
    `arxiv:${arxivID}`,
    download,
  ];

  const title = meta("citation_title");
  const date = meta("citation_online_date");

  const authors = Array.from(
    document.querySelectorAll("meta[name='citation_author']")
  ).map((x) => x.content);

  const year = date.substring(0, 4);
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
  const list = document.querySelector(".extra-services .full-text ul");
  const li = document.createElement("li");
  const a = document.createElement("a");
  //a.target = "_blank";
  a.innerHTML = "AKL IMPORT";
  a.href = url;
  li.appendChild(a);
  list.appendChild(li);
})();
