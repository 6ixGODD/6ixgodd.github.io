bwww Design and Implementation Brief
====================================

Status
------

This file is the authoritative design document for this repository.

It is intentionally not a marketing README.  An implementation agent should read this
file before changing the builder, the content format, the generated site structure, or
the deployment workflow.

The repository is a filesystem-first personal website.  The filesystem is the CMS, Git
is the history and publishing trigger, and ``bwww`` is the compiler from source content
to a static website.

Core idea
---------

The site should feel like a small collection of files exposed through HTTP, not like a
modern web application pretending to be a terminal.

The public site uses normal hyperlinks and normal browser behavior.  Visual styling is
minimal.  Information layout matters more than visual decoration.

The basic pipeline is::

    content/ + static/
            |
            v
          bwww
            |
            v
          dist/
            |
            +--> GitHub Pages (v1)
            +--> object storage / CDN (later)

The generated site has no application server, database, login system, admin panel,
hydration, frontend framework, or client-side router.

Technology choices
------------------

Rust
~~~~

The builder and its CLI are written in Rust.

Rust is used because this project is primarily a compiler-like filesystem tool:

* filesystem traversal;
* parsing and validation;
* typed intermediate models;
* deterministic rendering;
* path handling;
* static output generation;
* one small deployable binary.

The type system should encode invariants where practical.  Do not imitate enterprise
architecture by creating traits for every type.  Introduce a trait only when there is a
real substitution boundary or when testing materially benefits from it.

reStructuredText
~~~~~~~~~~~~~~~~

Human-authored textual content uses reStructuredText (RST).

The builder does **not** need to implement all of Docutils.  ``bwww`` owns a deliberately
small RST subset.  Add syntax only when real content needs it.

The v1 subset should support:

* document title and section headings;
* paragraphs;
* emphasis and strong emphasis;
* inline code;
* unordered and ordered lists;
* hyperlinks;
* literal/code blocks with a language hint;
* block quotes;
* images with alt text and optional caption;
* simple tables;
* math blocks rendered as native MathML;
* document metadata fields used by posts.

Unsupported syntax must produce an explicit diagnostic.  Never silently drop unknown
content.

HTML and CSS
~~~~~~~~~~~~

Generated HTML should be semantic and intentionally small.

The current visual language is:

* Times New Roman (or browser serif fallback) for prose;
* system monospace for navigation, metadata, indexes, code, and tables;
* approximately 14px base text;
* browser-native links;
* almost no decorative UI;
* no frontend framework;
* no icon library;
* external links may use the literal ``↗`` marker;
* native MathML rather than a JavaScript math runtime;
* images must be responsive and must never overflow the reading column.

The exact CSS is a renderer concern, but the site should remain readable if most CSS is
removed.

JavaScript
~~~~~~~~~~

JavaScript is optional enhancement only.

The first justified use is the homepage search/filter.  The site must remain navigable
without JavaScript through the archive and tag indexes.

JSON
~~~~

``serde`` and ``serde_json`` are used for generated machine-readable data such as the
search index.  JSON is generated output, not the CMS storage format.

Repository layout
-----------------

Expected layout::

    .github/
      workflows/
        pages.yml

    content/
      summary.rst
      resume.rst
      resume.pdf                 # optional, independent from resume.rst
      posts/
        YYYY-MM-DD-slug/
          index.rst
          image.webp             # optional local asset

    static/
      ...                        # files copied verbatim to the site root

    src/
      main.rs
      cli.rs
      error.rs
      commands/
        mod.rs
        build.rs
        check.rs
        new.rs
        serve.rs
      content/
        mod.rs
      rst/
        mod.rs
        ast.rs
        parser.rs
        render.rs
      site/
        mod.rs
        render.rs

    tests/
      fixtures/
        rst/
      site_build.rs

    dist/                        # generated; never committed
    dist.tmp/                    # temporary build output; never committed

``content/`` is source of truth.  ``dist/`` is disposable.

Content contracts
-----------------

Summary
~~~~~~~

``content/summary.rst`` supplies the short human introduction on the homepage.

It is intentionally concise.  It is not a biography database and does not need front
matter in v1.

Resume
~~~~~~

``content/resume.rst`` is the source for the HTML resume page.

The resume has its own renderer.  It may share the global page shell and typography, but
it must not be treated as a normal blog post.

``content/resume.pdf`` is optional and completely independent from ``resume.rst``.

If ``resume.pdf`` exists, the build copies it verbatim to ``dist/resume.pdf`` and the
HTML resume may expose a ``[pdf]`` link.  The builder must never generate the PDF from
RST and must never assume the PDF contains the same information as the HTML resume.

The RST source should also be exposed as a raw file from the resume page.

Posts
~~~~~

One post is one directory::

    content/posts/2026-09-23-why-i-inject-http-clients/
      index.rst
      diagram.webp

The directory date is organizational.  Parsed metadata is authoritative.

A post starts with an RST title followed by a field list::

    Why I inject HTTP clients
    =========================

    :date: 2026-09-23
    :slug: why-i-inject-http-clients
    :tags: python, infra
    :lang: en
    :draft: false

    Body starts here.

Required v1 fields:

``date``
    ISO ``YYYY-MM-DD`` publication date.

``slug``
    Stable URL slug.  It must be unique.

``tags``
    Comma-separated tags.  Zero tags are permitted, but empty tag names are not.

Optional fields:

``lang``
    BCP-47-ish language label used for generated HTML.  Default: ``en``.

``draft``
    ``true`` or ``false``.  Draft posts are validated but are not emitted into public
    indexes or output during a normal production build.

Do not add ``series``.  A series is simply a tag.

Public URL model
----------------

Use stable directory-style URLs::

    /                           homepage; newest N posts
    /posts/<slug>/             rendered post
    /posts/<slug>/index.rst    raw source
    /resume/                   rendered resume
    /resume/resume.rst         raw resume source
    /resume.pdf                independent PDF, if provided
    /tags/                     tag index
    /tags/<tag>/               posts with that tag
    /archive/                  year index
    /archive/<year>/           month index
    /archive/<year>/<month>/   posts in the month
    /feed.xml                  Atom feed
    /sitemap.xml               sitemap
    /search-index.json         generated search metadata

The canonical representation of an article should use the directory URL, not an
``.html`` URL.

Homepage behavior
-----------------

The homepage shows a fixed number of the most recent published posts.

The v1 constant is::

    HOMEPAGE_POST_LIMIT = 20

Do not paginate the homepage.

Older content is reached through ``archive/``.  This avoids unstable page-number URLs
where new posts continuously move older posts from page to page.

The homepage search input searches the generated metadata for **all** published posts,
not just the latest 20.  Searching title, date, and tags is sufficient for v1.  Full-text
search is explicitly out of scope.

Tags
----

Do not render a tag cloud on the homepage.

Tags are reachable from:

* the tag column in the post index;
* post metadata;
* the dedicated ``/tags/`` index.

If the tag set grows to hundreds of values, the homepage must remain unchanged.

Archive
-------

Archive structure is time-based::

    /archive/
      2026/
        09/
        08/
      2025/

``/archive/`` lists years and counts.  A year page lists months and counts.  A month page
lists posts.

There is no traditional numbered pagination in v1.

Assets and binary files
-----------------------

Images are the normal binary content of posts.

Local images should live beside the post that owns them.  Relative RST references are
resolved relative to the source document and copied into the corresponding generated
post directory.

Remote image URLs are allowed.  The builder must not download or mirror a remote image
unless a future feature explicitly adds that behavior.

Initial Git policy:

* images may be stored directly in Git;
* prefer WebP, AVIF, PNG, or JPEG;
* keep ordinary web images small; roughly 2 MiB per image is a useful soft ceiling;
* do not commit RAW camera files, PSD files, videos, model weights, datasets, or build
  artifacts;
* avoid repeatedly replacing large binary files when an immutable new asset is equally
  practical;
* Git LFS is not part of the v1 Pages workflow;
* if repository growth becomes annoying, move media to object storage without changing
  the content model: RST already permits remote URLs.

``.gitattributes`` marks common binary formats as binary so Git does not attempt textual
normalization or diffs.

CLI contract
------------

The binary is named ``bwww``.

Only four user-facing commands belong in v1.

``bwww new <title> [--slug <slug>]``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Create a new draft post directory and ``index.rst`` template.

The command is convenience only.  It must not commit, push, deploy, or maintain hidden
state.  A user must always be able to create the same files manually.

If a safe ASCII slug cannot be derived from the title, require ``--slug``.

``bwww check``
~~~~~~~~~~~~~~

Validate the source tree without producing a public site.

Eventually this command must detect at least:

* malformed RST;
* unsupported syntax;
* invalid or missing metadata;
* duplicate slugs;
* invalid dates;
* empty tag names;
* missing local images;
* invalid internal links when detectable;
* invalid source paths;
* unsafe output path traversal.

Diagnostics must identify the source file and, once the parser supports it, line and
column whenever possible.

``bwww build``
~~~~~~~~~~~~~~

Perform a production build into ``dist/``.

A build must be deterministic for a fixed source tree, builder version, and configuration.

The build owns ``dist/`` and may remove/recreate it.

Expected build phases:

1. discover source files;
2. parse metadata and RST into typed models / AST;
3. validate all documents and cross-document invariants;
4. sort published posts;
5. derive tag and archive indexes;
6. derive search index, feed, sitemap, and SEO metadata;
7. render semantic HTML;
8. copy local assets and raw RST sources;
9. copy ``static/`` verbatim;
10. copy ``content/resume.pdf`` verbatim if present;
11. write ``dist/.nojekyll`` for GitHub Pages.

Do not partially publish a site after validation errors.

``bwww serve [--port 8080]``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Build the site and serve ``dist/`` on localhost.

Hot reload is not required in v1.  Rebuilding and refreshing the browser is acceptable.
The server exists for local preview only and must not become an application runtime.

Error policy
------------

Failures are explicit.

Do not silently substitute empty values for malformed content.  Do not catch broad
errors merely to keep building.  If a source document cannot be understood, identify the
document and fail the build.

Filesystem and parser code must reject path traversal.  A source reference must never
escape the intended content or output roots accidentally.

Keep error types small and meaningful.  Use ``thiserror`` for owned error enums.  Add
``anyhow`` only if there is a demonstrated need for a top-level opaque error context; it
is not required by default.

Rust implementation rules
-------------------------

* ``cargo fmt`` defines formatting.
* ``cargo clippy --all-targets -- -D warnings`` should pass before merge.
* Prefer owned domain structs over nested ``HashMap<String, Value>`` data plumbing.
* Parse external/source text at an explicit boundary and convert it into typed internal
  models.
* Keep rendering pure where practical: model in, bytes/string out.
* Separate path discovery, parsing, validation, derived indexes, and rendering.
* Avoid global mutable state.
* Avoid hidden environment configuration in core code.
* Do not create a trait for a type that has only one implementation unless the boundary
  is useful by itself.
* Do not add async Rust unless a real concurrent I/O use case appears.  v1 is a local
  filesystem compiler and can remain synchronous.
* Never trust a source path simply because it originated inside the repository.
* Generated HTML must escape text by default.  Only renderer-owned markup may bypass
  escaping.

Suggested internal models
-------------------------

The exact names may evolve, but the architecture should converge toward typed values
similar to::

    Site
    Post
    PostMeta
    Resume
    TagIndex
    ArchiveIndex
    SearchEntry
    DocumentAst

Do not use a generic ``Document = HashMap<String, String>`` as the permanent model.

Testing strategy
----------------

Use unit tests for:

* slug creation;
* metadata parsing;
* each supported RST construct;
* HTML escaping;
* path normalization;
* archive grouping;
* tag grouping;
* search-index generation.

Use fixture/golden tests for renderer output where useful.  Keep fixtures small and
human-readable.  ``tests/fixtures/rst/`` is reserved for parser cases.

A future end-to-end test should build a small fixture site into a temporary directory and
assert the expected output tree.

Git workflow
------------

Git is the CMS history and the publishing trigger.

Normal post workflow::

    bwww new "Why I inject HTTP clients"
    $EDITOR content/posts/2026-09-23-why-i-inject-http-clients/index.rst
    bwww check
    bwww serve
    git add content/
    git commit -m "content: add HTTP client DI note"
    git push

Normal HTML resume workflow::

    $EDITOR content/resume.rst
    bwww check
    bwww serve
    git add content/resume.rst
    git commit -m "resume: update skills"
    git push

PDF resume update::

    cp ~/Downloads/resume.pdf content/resume.pdf
    git add content/resume.pdf
    git commit -m "resume: update downloadable PDF"
    git push

No special publish command is required.  Publishing is ``git push`` followed by CI.

Commit convention
-----------------

Use a small conventional prefix vocabulary.  A commit should represent one coherent
change.

``content:``
    Add or edit normal posts and text content.

``resume:``
    Change resume RST or the downloadable PDF.

``site:``
    Change builder behavior, parsing, rendering, indexes, or site layout.

``assets:``
    Add or change media without a substantive content change.

``fix:``
    Fix a builder or generated-site bug.

``ci:``
    Change GitHub Actions or deployment behavior.

``test:``
    Add or change tests/fixtures only.

``chore:``
    Repository maintenance that does not fit the above.

Examples::

    content: add CUDA memory hierarchy note
    resume: update skills
    site: generate monthly archive indexes
    fix: escape post titles in search results
    ci: deploy dist with GitHub Pages action

Keep subjects concise.  Do not commit ``dist/`` or ``target/``.

This is a binary crate, so ``Cargo.lock`` should be committed once it is generated by the
first real Cargo invocation.  Do not hand-edit the lockfile.

GitHub Pages deployment
-----------------------

The initial deployment target is GitHub Pages using GitHub Actions.

``.github/workflows/pages.yml`` is responsible for:

1. checkout;
2. selecting stable Rust;
3. formatting / clippy / tests;
4. ``bwww check``;
5. ``bwww build``;
6. uploading ``dist/`` as the Pages artifact;
7. deploying the artifact.

The ``main`` branch is the production source branch for v1.

The deployment system must consume only ``dist/``.  Deployment logic must not leak into
content parsing or rendering.

Object storage later
--------------------

A future target may upload ``dist/`` to S3-compatible object storage, Alibaba OSS, or a
similar static host.

That change should be a CI/deployment concern.  It must not require changing post files
or the builder's content model.

A future ``bwww asset`` helper may optimize and upload large images to object storage,
but it is explicitly out of scope for v1.

Current implementation
----------------------

The Rust builder now validates and renders the repository content.  ``bwww check``
parses every post, including drafts.  ``bwww build`` writes the complete site to
``dist/`` only after validation succeeds.  Published posts, raw RST, local images,
resume, tags, archive, search metadata, feed, sitemap, and ``.nojekyll`` are emitted.

The RST parser intentionally supports the subset listed above.  Code blocks use
``.. code-block:: language``; images use ``.. image:: path`` with required ``:alt:``
and optional ``:caption:``; math blocks use basic identifiers, numbers, and operators.
For more complex RST or math, extend the parser with tests before adding that content.

The checked-in ``content/posts/example/`` is a draft and will not appear on the
published site.  ``bochen-site-v6-resume-reference/`` is a local visual reference
and is excluded from Git and the published site.

Local commands::

    cargo run -- check
    cargo run -- build
    cargo run -- serve --port 8080
    cargo run -- new "A post title" --slug a-post-title

For a non-default hosting URL, pass ``--site-url`` to ``build`` or ``serve``.
It must be an HTTPS URL ending in ``/``.  The URL is used for feed and sitemap;
page navigation uses relative links and works under a GitHub Pages project path.
Write internal RST hyperlinks as relative paths for the same reason.

Publishing to GitHub Pages
~~~~~~~~~~~~~~~~~~~~~~~~~~

1. Put this directory in a GitHub repository with ``main`` as the production branch.
2. Under repository Settings → Pages → Build and deployment, choose
   **GitHub Actions** as the source.
3. Push ``main``.  ``.github/workflows/pages.yml`` validates, builds, uploads
   only ``dist/``, and deploys it.  It derives the canonical URL from the GitHub
   repository name, including the project path when applicable.
4. Open the URL shown by the workflow's deploy job.

The workflow includes hidden files in the Pages artifact so ``.nojekyll`` is
published.  No domain purchase or OSS configuration is needed for v1.

Implementation order for Codex
-------------------------------

Unless a concrete task says otherwise, implement in this order:

1. define typed content metadata and source-path models;
2. implement source discovery and path safety;
3. implement the constrained RST lexer/parser and AST;
4. implement metadata validation and ``bwww check``;
5. implement semantic HTML rendering for RST;
6. implement ``Post`` and resume renderers using the agreed visual language;
7. implement tag indexes and year/month archives;
8. implement homepage latest-20 behavior;
9. implement ``search-index.json`` and small client-side metadata search;
10. implement Atom feed and sitemap;
11. add golden/end-to-end tests;
12. remove or rename bootstrap-only rendering code.

Do not expand scope to a database, CMS UI, frontend framework, or deployment dashboard.
