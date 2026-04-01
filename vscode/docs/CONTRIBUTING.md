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

### Publishing the extension

For details on how the extension is published, reference [the VS Code
docs on publishing extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension).

Note that when running `vsce publish`, you will need to pass
`--follow-symlinks` so that it can find the icons.

#### OpenVSX

In order to publish the extension for Cursor and other VS Code derivatives, you
will need to publish on OpenVSX as well. Follow
[these instructions](https://github.com/eclipse-openvsx/openvsx/wiki/Publishing-Extensions)
to publish there.

Note that because `--follow-symlinks` is needed, you will need to run
`vsce package`, then run `ovsx publish --packagePath oneil-0.2.0.vsix`.

> ![WARNING]
> At the time of writing, Open VSX does not allow you to set a duration for your
> PAT (Personal Access Token). For this reason, it is recommended to delete you
> PAT after using it. This ensures that it is useless if stolen.
>
> There are many cases of malware being published using a non-expiring token
> a couple years after the user forgot about it.
