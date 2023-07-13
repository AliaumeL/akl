https: function logURL(requestDetails) {
  const url = new URL(requestDetails.url);
  if (url.host == "pdf.sciencedirectassets.com") {
    const pii = url.searchParams.get("pii");
    const obj = {};
    obj[pii] = requestDetails.url;
    browser.storage.local.set(obj);
    console.log(`AKL Storing: ${requestDetails.url}`);
  }
}

browser.webRequest.onBeforeSendHeaders.addListener(logURL, {
  urls: ["https://pdf.sciencedirectassets.com/*"],
});

browser.runtime.onMessage.addListener(async (request, sender, sendResponse) => {
  // A request is an object
  // with a field 'type' which is a string
  // that determines how the rest of the request is parsed
  console.log(sender);
  console.log(request);

  //const type = request.type;
  //if (type === "download-file") {
  //} else if (type === "resolve-uri")  {

  //}

  const dwnl = await browser.downloads.download({
    url: request.url,
    filename: request.title,
    conflictAction: "overwrite",
  });

  // FIXME: we should wait for the download to be finished
  // using the onChanged event handler
  return browser.downloads.search({ id: dwnl });
});
