(function () {
  const DOI = document.querySelector(
    `meta[name="dc.Identifier"][scheme="doi"]`
  ).content;

  const list = document.querySelector("li.pdf-file").parentElement;
  const li = document.createElement("li");
  const a = document.createElement("a");
  const i = document.createElement("i");
  const s = document.createElement("span");

  s.innerHTML = "AKL";
  a.className = "btn green-text";
  a.disabled = true;
  li.className = "pdf-file";

  a.appendChild(i);
  a.appendChild(s);

  li.appendChild(a);
  list.appendChild(li);

  fetch("https://dl.acm.org/action/exportCiteProcCitation", {
    body: new URLSearchParams({
      dois: DOI,
      targetFile: "custom-bibtex",
      format: "bibTex",
    }),
    method: "POST",
  }).then(async (response) => {
    i.className = "icon-e-Reader";
    const json_global = await response.json();
    const json = json_global.items[0];
    const key = Object.keys(json)[0];
    const data = json[key];

    const authors = data.author.map((a) => `${a.given} ${a.family}`);

    console.log(data);
    const conference = data.issue || data["collection-title"];
    const download = `https://dl.acm.org/doi/pdf/${DOI}`;
    const identifiers = [
      `https://dx.doi.org/${DOI}`,
      `doi:${DOI}`,
      `https://dl.acm.org/doi/${DOI}`,
      download,
    ];

    const title = data.title;
    const year = parseInt(data.issued["date-parts"][0]);
    const context = [];
    if (conference) {
      context.push(conference);
    }

    a.onclick = () => {
      browser.runtime
        .sendMessage({
          url: download,
          title: encodeURIComponent(`${DOI}.pdf`),
        })
        .then((download_items) => {
          i.className = "icon-pdf-file";
          const payload = {
            uri: download_items[0].filename,
            title,
            authors,
            context,
            identifiers,
            view: true,
            force: false,
          };
          const query = new URLSearchParams({
            payload: JSON.stringify(payload),
          });
          const url = "akl://import-document/?" + query.toString();
          a.href = url;
          a.onclick = undefined;
        });
    };
  });
})();
