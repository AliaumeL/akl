# The Firefox Extension

The main idea is taken from the `zotero-connector` extension,
see [Zotero Connectors].

A [Firefox extension] is defined by a `manifest.json` file that lists the
permissions required for the extension (performing web requests, accessing
local storage, lists of urls that can be intercepted, etc.), together with
a list of [Background Scripts], that are run independently of any open
tab, and [Content Scripts], that are run on a per-tab basis.

The dichotomy is made to allow a separation of concerns and run long
processes on the background rather than on the current tab's javascript
runtime, but also for safety. This explains why some of the API's provided
differ between [Background Scripts] and [Content Scripts], in particular
with respect to intercepting web requests, and communicating with external
processes.


## Code Organisation

For now, the code is organised as follows: one [Content Script] per
website "family", that creates a suitable `import-document` URL scheme,
i.e., of the form
`akl://import-document/?payload=URLENCODED-JSON-OBJECT`
where the `JSON-OBJECT` contains the following keys:

- uri: a direct url/filepath to the pdf, no redirection or authentication will be done when following this url.
- title: a title for the document 
- authors: a list of authors 
- context: a list of "contextual informations" (conference name, erratum, etc.)
- identifiers: a list of "identifiers" (DOI, arxiv_id, hal_id, etc.)
- view: a boolean specifying whether the file should be opened after import
- force: a boolean specifying whether the file should be overwritten if it already is present in the library

Because `science direct` is a terrible website, the direct pdf urls are
not easily extracted from the webpage itself. For that reason,
a [Background Script] was created to listen to the web requests of the
online pdf viewer to get the desired URL.

## Future Work

It could be interesting that the extension itself downloads the file
rather than passing a URL to `akl`, and allow users to import files that
have already been downloaded. This implies moving more of the logic to the
[Background Script]. Furthermore, to have visual indicators of the
download status (and see if the document already exists in the library),
it is necessary to communicate with the `akl` program directly.

[Zotero Connectors]: https://github.com/zotero/zotero-connectors
[Firefox extension]: https://developer.mozilla.org/fr/docs/Mozilla/Add-ons/WebExtensions
[Background Scripts]: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Background_scripts
[Background Script]: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Background_scripts
[Content Scripts]: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Content_scripts
