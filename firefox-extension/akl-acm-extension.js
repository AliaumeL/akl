setTimeout(() => {
  const DOI = document.querySelector(
    `meta[name="dc.Identifier"][scheme="doi"]`
  ).content;

  const url_params = {
    dois: "10.1145/298514.298591",
    format: "bibTex",
    targetFile: "custom-bibtex",
  };

  fetch("https://dl.acm.org/action/exportCiteProcCitation", {
    body: new URLSearchParams({
      dois: DOI,
      targetFile: "custom-bibtex",
      format: "bibTex",
    }),
    method: "POST",
  }).then(async (response) => {
    const json_global = await response.json();
    const json = json_global.items[0];
    console.log(json);
    const key = Object.keys(json)[0];
    console.log(key);
    const data = json[key];
    console.log(data);

    const authors = data.author.map((a) => `${a.given} ${a.family}`);

    const conference = data["collection-title"];
    const download = `https://dl.acm.org/doi/pdf/${DOI}`;
    const identifiers = [
      `https://dx.doi.org/${DOI}`,
      `doi:${DOI}`,
      `https://dl.acm.org/doi/${DOI}`,
      download,
    ];

    const title = data.title;
    const year = data.issued["date-parts"][0];
    const context = [conference];

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
    const list = document.querySelector("li.pdf-file").parentElement;
    const li = document.createElement("li");
    const a = document.createElement("a");
    const i = document.createElement("i");
    const s = document.createElement("span");

    i.className = "icon-pdf-file";
    s.innerHTML = "AKL";
    a.className = "btn green-text";
    a.href = url;
    li.className = "pdf-file";

    a.appendChild(i);
    a.appendChild(s);

    li.appendChild(a);
    list.appendChild(li);
  });
}, 100);
