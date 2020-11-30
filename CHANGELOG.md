# 1.2.1 (2020-11-24)

## :bug: Fixes

* Fixed bug where the shell was not processing escape sequences in user input.

# 1.2.0 (2020-11-13)
## :tada: Enhancements
* Added `clear` to clear the screen.
* Remove pyqldb as dependency and add driver submodule.

# 1.1.0 (2020-10-22)

## :bug: Fixes

* Fixed empty line repeating the last command used in the shell. Now, an empty line does not repeat the last command.

## :tada: Enhancements

* Added `quit` and `Ctrl + D` as ways to exit the shell along with existing `exit` command.
* Added interactive and non-interactive transactions to the shell. More details can be found in the [release notes](http://github.com/awslabs/amazon-qldb-shell/releases/tag/v1.1.0).
* Changed shell depedency from Cmd to prompt-toolkit to pasting multiple lines and entering multiple lines. More details can be found in the [release notes](http://github.com/awslabs/amazon-qldb-shell/releases/tag/v1.1.0).
* Added support for recommendation on keywords and active Table names on pressing the `Tab` key.
* `Ctrl + C` cancels the current command.

# 1.0.1 (2020-07-01)

## :bug: Fixes

* Error out of shell immediately when no credentials are present as reported in [issue#14](https://github.com/awslabs/amazon-qldb-shell/issues/14)
* Lock driver and amazon ion versions during shell installation

# 1.0.0 (2020-04-20)

## :tada: Enhancements

* Relax boto version requirement on installation.

## :book: Documentation

* Add unit tests.
* Correct quit to exit on README.

# 0.1.0 (2020-03-14)

* Preview release of the shell : The release supports single line statements and couple of experiences may change in future with the addition of new features.
* Connect using 'qldbshell --region <region_code> --ledger <ledger_name>'.
* Interact with the ledger via the shell using a [PARTIQL](https://partiql.org/docs.html) statement per line.
* [Query Ion with PARTIQL](https://docs.aws.amazon.com/qldb/latest/developerguide/ql-reference.query.html).

