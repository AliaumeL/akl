# AKL

**Warning** This is early stage software. Do not install unless you know what
you are doing.

## Overall Idea

The goal of this project is to simplify the navigation through a PDF library.
A typical usage is:

1. The user opens a PDF of interest
2. The PDF contains links to other PDFs online, sometimes with precise
   destination information (also known as `named destinations` in the PDF
   lingo).
3. The user wants to click on these links and the correct PDF file should
   open, using the following requirements
   1. The pdf should be opened using the default pdf viewer
   2. The pdf should not be downloaded twice, so that annotations are preserved
   3. The pdf should be opened at the correct position (if specified)

### Implementation

The main idea is that `akl` acts as a proxy when opening a PDF file.

1. The first time a pdf is opened, a duplicate is created
   with all the (external) links rewritten to use a custom
   url scheme handler
2. Then, the default pdf viewer of the system is used to open
   this duplicated version.
3. When external links are clicked on, the operating system
   forwards them to the `akl` program.
4. If the links are already "connected" to a real pdf file
   on the system, the program opens it as in step (1). Otherwise,
   the program delegates the link to the default program (most of the time,
   the default web browser is used).

For all these steps to work correctly, one needs the following things

a. A way to build the new pdfs with rewritten links
b. A way to tell the operating system that `akl` is handling a particular
   url scheme
c. A way to import pdfs, a cache for the rewriten pdfs, and basically
   a pdf database.

Non goals are 

1. Automatically importing links. While it seems like a good idea,
   it should be the job of another program, typically a browser
   extension.
2. Graphical interface. The objective is to have a very simple
   program that interfaces as smoothly as possible to widely
   different environments, hence leveraging as much as possible
   external programs.


## How To Install The AKL-RS program

For now, you have to build the file from source, which means running
the following command.

```bash
cargo build --release
```

Note that depending on your system, you may need to install the following:

- `cargo`
- `pkg-config`
- `libssl-dev`
- `libdbus-1-dev`

### On Linux

Add the desktop file to your list of applications.

```bash
cp dist/akl-opener.desktop ~/.local/share/applications/akl-opener.desktop
```

Register the desktop file as able to open `akl` links.

```bash
echo "x-scheme-handler/akl=akl-opener.desktop;" >> ~/.local/share/applications/mimeapps.list
```

Add the `akl` binary to your path, for instance by running

```bash
cp target/release/akl-rs /usr/local/bin/akl
```

If for some reason on `evince` you do not have the right to launch applications,
this can help: in `/etc/apparmor.d/usr.bin.evince`, add a line
allowing to launch `/usr/local/bin/akl` via:

```
/usr/local/bin/akl ux,
```

Beware that this means you trust `akl` to run arbitrary programs.
For the configuration to take effect, you must use

```
apparmor_parser -T -W -r /etc/apparmor.d/usr.bin.evince
```


### On OSX

TODO

### On Windows

Copy the binary `akl-rs.exe` into a safe location,
for instance `C:\Program Files (x86)\AKL\akl.exe`.

Optionally, if you want to be able to access it directly from the command line,
you can add this directory to your PATH:
- WIN+R
- Enter `SystemPropertiesAdvanced.exe`
- Environment variables -> Path (current user or system) -> Edit -> New
- Enter `C:\Program Files (x86)\AKL\`

Then, register the custom URI scheme by running `dist/register_akl_machine.reg` (registration for all users)
or `dist/register_akl_user.reg` (registration for the current user only). If your binary is not in `C:\Program Files (x86)\AKL\akl.exe`,
you have to edit the `.reg` file accordingly.

## How To install the AKL Extension

For now, the web extension only works with Firefox
and is not available in the app-store. To load the extension
you can go to `about:debugging`, select "my firefox", and
temporarily load the `firefox-extension/manifest.json` file.

