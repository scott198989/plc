# Render-Text Digest Recomputation

## Finding

The adversarial-audit Word lead records this value as a combined normalized-text
SHA-256:

    DD5FAA0B213307CC6D4FBB8D5087FBC59751B777009CD2997BA9EF453E02FF

It contains 62 hexadecimal characters and therefore is not a valid SHA-256
representation. No inference is made about how the malformed value arose.

The named script in this directory recomputes the digest from the exact fresh
render artifact used for the audit page anchors. The accepted 64-character
result under the recorded procedure is:

    44B089F87E65B2FC6A2D40DEA9D19B326A252E28CEA846E119A9381A5A0B1728

## Procedure

For each PDF page, pdfplumber uses
extract_text(x_tolerance=2, y_tolerance=2). The result has soft hyphens
removed, newlines replaced with spaces, ASCII alphabetic line-break
hyphenation joined, text lowercased, every non-[a-z0-9] run replaced by one
space, and surrounding page whitespace stripped. The 40 normalized pages are
joined by one LF with no terminal LF and encoded as UTF-8 before SHA-256.

The fresh audit render and the stored local render have different PDF bytes but
the same 40 normalized page strings and the same normalized-text digest. The
raw command, versions, paths, byte lengths, artifact hashes, and outputs are in
render-text-digest-raw-output.txt.

## Evidence boundary

Python 3.12.13 and pdfplumber 0.11.9 were used because they reproduce the
existing audit helper's extraction procedure. They are evidence-reproduction
tools, not the pinned Python 3.13.12 governance-verifier runtime. This record
does not by itself admit those tools, approve visual QA, or prove that either
PDF is byte-identical to the DOCX.
