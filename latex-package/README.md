# AKL LaTeX Package

This package is a work in progress to simplify the
construction of LaTeX documents that have enriched citations.

## How to use

In the preamble of your LaTeX document, add the following line.

```latex
\usepackage[rewrite]{akltex}
```

The option `rewrite` is placed so that enriched citations
are available for working documents. When producing a PDF
that should be distributed, remove this option.

Exposes two main functions, `aklset` and `aklget` that
are made to store and access enriched links.

```latex
\aklset{bibtex-key}{written-name}{full-url}
\aklget{bibtex-key}{written-name}
```

It is also possible (and recommended) to use the key-value version of the
`aklset` command

```latex
\akldef{
    key=bibtex-key,
    name=written-name,
    url=full-url
}
```

To better interface with citations in LaTeX,
an `aklcite` command is defined and can be used
as follows

```latex
\aklcite[written-name]{bibtex-key}
% which expands to
\cite[\aklget{bibtex-key}{written-name}]{bibtex-key}
```

Sometimes, it is needed to reference several parts of the same document. In
this case, it may be easier to use a regular `cite` command together with
the `withakl` scoping mechanism.

```latex
\cite[\withakl{bibtex-key}{
see \akl{Theorem 1} and \akl{Proposition 3}
}]{bibtex-key}
```

To avoid repeating the citation key in such situations,
a command `acite` is provided and can be used as follows.

```latex
\acite[see \akl{Theorem 1} and \akl{Proposition 3}]{bibtex-key}
```

## How to install

Simply place the file `akltex.sty` in the root directory of your
project.

## FAQ

### How can I refer to specific page numbers?

You can create custom destinations for pages as follows

```latex
\akldef{
    key=bibtex-key,
    name=page 4,
    url=url-to-document?page=4
}
```

And use it with `\aklcite[page 4]{bibtex-key}`,
or `\acite[on \akl{page 4} and \akl{page 7}]{bibtex-key}`.
