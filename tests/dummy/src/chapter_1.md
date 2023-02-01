# Chapter 1

Here is the Rust logo:

![Rust Logo](rust-logo.png)

Listing example:
{{#rustdoc_include ../listings/ch02-guessing-game-tutorial/no-listing-04-looping/src/main.rs:here}}

<img alt="Rust Logo in html" src="rust-logo.svg" class="center" style="width: 20%;" />

The following straight quotes should transform into curly quotes:

"One morning, when Gregor Samsa woke from troubled dreams, he found himself 'transformed' in his bed into a horrible vermin."

## Footnote

### [Pop-up Footnotes][pfn]

In EPUB 3 Flowing and Fixed Layout books, you can create pop-up footnotes[^testfootnote] by labeling footnotes with the appropriate epub:type values. You use two elements to create a pop-up footnote: an anchor (`<a>`) element that triggers the popup and the `<aside>` element that contains the footnote text. Both elements have an epub:type attribute to identify their purpose: epub:type="noteref" to trigger the popup and epub:type="footnote" to indicate the footnote’s text.
footnote in a flowing book

In the example below, the anchor element (`<a>`) has two attributes: epub:type="noteref" and a link that references the location of the element that contains the popup's text.

The `<aside>` element that contains the popup's text also has two attributes:

    id="myNote" that matches the value of the href attribute in the link that references it

    epub:type="footnote"

Because the `<aside>` element has an epub:type of footnote, the text is hidden in the main body of the book. The text will only be seen by the reader in the context of the popup.

```html
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">. . .<p> <a href="chapter.xhtml#myNote" epub:type="noteref">1</a></p><aside id="myNote" epub:type="footnote">Text in popup</aside>. . .</html>
```

Note: Use of the epub:type attribute requires the inclusion of the namespace xmlns:epub="http://www.idpf.org/2007/ops in the `<html>` element.

If your book requires a specific text direction, such as right-to-left, and you want the footnote text direction to match, wrap the footnote text in a `<p>` element and add a style to specify the text direction:

```html
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"> . . .<p> <a href="chapter.xhtml#myNote" epub:type="noteref">1</a></p><aside id="myNote" epub:type="footnote"><p style="direction:rtl">Text in popup</p></aside>. . .</html>
```

Note: When adding pop-up footnotes in EPUB 3 books, you can replace the `<aside>` element with a `<div>` or `<p>` element. Use the `<aside>` element when you want to hide the footnote; use a `<div>` or `<p>` element when you want the footnote to appear in the normal reading view. If you use `<div>` or `<p>` and a user clicks the footnote link, the content appears in a popup, but the footnote is also visible as part of the text on the page.

### [Footnotes and Asides][Footnotes_n_Asides]

Where the notes are displayed as footnotes then superscripted numbers can also be used, but some publishers use the less than useful range of symbols in the order set out here. *, †, ‡, §, ‖,¶. Once all of these symbols are used on the page, then the continuation uses the same but doubled up like §§.

The convention is to use these symbols in a particular order on the page as seen in this example, Journey to Britain, Bronwen Riley

part of a page from *Journey to Britain*, Bronwen Rileypart of a page from *Journey to Britain*, Bronwen Riley

Unusually, both symbols and numbers can be used as in this example from The Devils Details, Chuck Zerby

a page from *The Devils Details*, Chuck Zerbya page from *The Devils Details*, Chuck Zerby

In this case, symbols are used for general notes and the numbers are used for citations.


[^testfootnote]: "Neque porro quisquam est qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit... There is no one who loves pain itself, who seeks after it and wants to have it, simply because it is pain..."

[pfn]: https://help.apple.com/itc/booksassetguide/en.lproj/itccf8ecf5c8.html
[Footnotes_n_Asides]: https://www.publisha.org/papers/footnotes/