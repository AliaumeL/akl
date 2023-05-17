setTimeout(() => {
  "use strict";

  const meta = (field) =>
    document.querySelector(`meta[name="${field}"]`).content;

  const DOI = meta("citation_doi") || meta("dc.identifier");

  const citation_url = meta("citation_pdf_url");
  const pii = citation_url.match(/pii\/(\w+)/)[1];
  console.log(pii);

  const download = document.querySelector("#save-pdf-icon-button")?.href;

  const identifiers = [`doi:${DOI}`, `https://dx.doi.org/${DOI}`];

  const crossrefRequest = new Request(`https://api.crossref.org/works/${DOI}`, {
    method: "GET",
    headers: { accept: "application/json" },
  });

  fetch(crossrefRequest)
    .then(async (response) => {
      const json = await response.json();
      const durl = await browser.storage.local.get();
      console.log(durl[pii]);

      const title = json.message.title[0];
      const authors = json.message.author.map(
        (author) => `${author.family}, ${author.given}`
      );
      const publisher = json.message["container-title"];
      //const date = json.message["published-online"]["date-parts"][0];
      const year = 2008;
      const context = [];

      const runtimeAnsw = await browser.runtime.sendMessage({
        pii,
      });

      const query = new URLSearchParams({
        payload: JSON.stringify({
          uri: runtimeAnsw["filepath"],
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
      const list = document.querySelector(".toolbox-panel");
      const btn = document.createElement("div");
      list.prepend(btn);
      btn.className = "save-pdf-button-wrapper";
      const a = document.createElement("a");
      a.className = "icon-button";
      a.innerText = "AKL";
      a.href = url;
      //a.target = "_blank";
      a.alt = "AKL IMPORT";
      btn.appendChild(a);
    })
    .catch((error) => {
      console.log(error);
    });
}, 200);
