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

## How To Install

### On Linux

1. Open the file `~/.local/share/applications/mimeapps.list` and add the
   following line `x-scheme-handler/akl=akl-opener.desktop;`
2. Add the file `dist/akl-opener.desktop` to your desktop files, for
   instance in `~/.local/share/applications/`
3. Add `akl` to your path

### On OSX

TODO

### On Windows

TODO
