# Contributing

Thank you for your interest in contributing to the Oneil VS Code extension! This extension provides language support for the Oneil programming language, enhancing the development experience with features like syntax highlighting and more to come. Whether you want to report bugs, suggest features, improve documentation, or contribute code, your help is greatly appreciated. This guide will help you get started with contributing to the project.

## Developing

### Running the extension locally

In order to run/debug the extension locally, follow the instructions from the
[VS Code docs](https://code.visualstudio.com/api/get-started/your-first-extension):

> Inside the editor, [ ... ] press `F5` or run the command > `Debug: Start
> Debugging` from the Command Palette (`Ctrl+Shift+P`). This will > compile and
> run the extension in a new Extension Development Host window.

### Syntax highlighting

In order to modify the syntax highlighting, edit
`syntaxes/oneil.tmLanguage.json`. For more details, see the [VS Code docs on
syntax highlighting](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide).

Use this [TextMate Grammar Testing Tool](https://www.leskoff.com/s02050-0) to
quickly iterate on the grammar.

### Publishing the extension

The extension can be published by getting an access token as described in
[the VS Code docs](https://code.visualstudio.com/api/working-with-extensions/publishing-extension).
It is recommended that you set an expiration for the token so that it can't
be stolen and abused in the future.

You will also need to ensure that you are part of the `careweather`
organization, which is managed by `oneil@careweather.com`. Talk to Patrick
about getting access to this email if you need it.

Once you have a Personal Access Token (PAT), run

```bash
vsce login
```

and provide your PAT. You can then run

```bash
vsce publish --follow-symlinks
```

to publish the extension to the market place.

> [!INFO]
> `--follow-symlinks` is required because the `icons` directory is a symlink to
> the docs in the parent `oneil` directory.

For details on how the extension is published, reference [the VS Code
docs on publishing extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension).

#### OpenVSX

In order to publish the extension for Cursor and other VS Code derivatives, you
will need to publish on OpenVSX as well. Follow
[these instructions](https://github.com/eclipse-openvsx/openvsx/wiki/Publishing-Extensions)
to get a Personal Access Token (PAT) for the Open VSX Registry.

> [!NOTE]
> You will need to be a member of the `careweather` namespace. To do so, see
> [these instructions for gaining access to a namespace](https://github.com/eclipse-openvsx/openvsx/wiki/Namespace-Access).
> Note that you will have to contact an owner of the `careweather` namespace to
> be added as a member.

Run

```bash
ovsx login
```

providing your PAT. Then, run

```bash
vsce package --follow-symlinks
```

to build the extension. A file should appear as `oneil-x.y.z.vsix` in the
current directory. This is the extension. Finally, run

```bash
ovsx publish --packagePath oneil-x.y.z.vsix
```

to publish to the Open VSX Registry.

> [!INFO]
> Normally for an extension, `ovsx publish` would be sufficient without having
> to manually build the package. However, because building the package requires
> the `--follow-symlinks` flag, we are required to take a roundabout way to
> publish.

<!---->

> ![WARNING]
> At the time of writing, Open VSX does not allow you to set a duration for your
> PAT (Personal Access Token). For this reason, it is recommended to delete you
> PAT after using it. This ensures that it is useless if stolen.
>
> There are many cases of malware being published using a non-expiring token
> a couple years after the user forgot about it.

### Testing the extension before publishing

Once a package version is published, it can't be overwritten. So it's important
to verify that the extension behaves as expected _before_ publishing it.
Otherwise, you will need to publish a whole new version with fixes (see
`v0.2.0` and `v0.2.1`).

To build the extension, run

```bash
vsce package --follow-symlinks
```

This will produce a file in the current directory named `oneil-x.y.z.vsix`.

Then, open up the command palette (`Ctrl+Shift+P`) and run
`Extensions: Install from VSIX...`. Select the file created by `vsce package`.

The extension is now installed in your editor. Test it and ensure that
everything works as expected.
