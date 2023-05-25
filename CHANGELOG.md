# Changes

## Version 0.28.2

* Switch from OpenSSL to RusTLS for better portability.
* Fix the verbose option.

## Version 0.28.1

* Updated dependencies.

## Version 0.28.0

* Disable the footnote for now. It did not render with Pantheon.

* Remove the parentheses around ticket IDs.

## Version 0.27.1

* Use a different footnote attribute, as recommended by the official `asciidoctor` documentation.

  Instead of the `{PrivateFootnote}` attribute containing the footnote text, it is now the `fn-private` attribute that contains the whole footnote macro.

## Version 0.27.0

* Private ticket IDs now feature a footnote that explains why the ID is not clickable. You can override the footnote text using the `{PrivateFootnote}` attribute.

## Version 0.26.1

* The tool now recognizes the legacy `cizrna/` directory path for compatibility with previous releases.

## Version 0.26.0

* Renamed from Cizrna to aCoRNs.
