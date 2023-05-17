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

browser.runtime.onMessage.addListener((request, sender, sendResponse) => {
  console.log(sender);
  console.log(request);
  const dwnl = await browser.downloads.download({
    url: "http://google.com",
      conflictAction: "overwrite",
  });
  sendResponse({ filepath: dwnl.filename });
});
