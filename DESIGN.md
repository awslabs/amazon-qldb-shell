# QLDB CLI Shell

## About

This is the specification of the QLDB Shell containing all features, commands, and options.

## Tenets

**Unless you know better ones.**

* We adhere to the principle of least astonishment
  (https://en.wikipedia.org/wiki/Principle_of_least_astonishment). That is, a
  shell should behave the way most customers expect it to behave, which will be
  very similar to the way in which all the other shells they've ever used
  behaves.
    * This isn't just “intention". During a design review for a new feature if
      something "surprises" you, that is a good indicator it's probably not
      right.
* We don't need to invent unless it doesn't exist.
    * Database shells have been around for more than 20 years. In most cases we
      should just do whatever it is that the MySQL/Postgres shells do.
* We favor compatibility with other shells, so for example, I can pipe the
  output of the QLDB shell to another program, like jq.

## CLI

We support using the shell as an executable which can receive input and pipe
output to other applications, so for example:

```sh
cat my-script.sql > qldbshell -f json -e | jq
```

Would pipe the results of executing the script in the script file into the jq
program. All possible arguments are:

```sh
Usage:
-m --display-metrics [on|off] # default: on
-f --format [ion|json|table] # default: ion

-a --auto-commit [on|off] # default: on

-p --endpoint endpoint # default: taken from credentials
-l --ledger ledger # default: none

-P --prompt your-prompt-syntax # default: qldb>
-d --delimiter your-delimiter # default: \n
-o --output [STDOUT|file] [outfile]? # default: console

-h --history [on|off] [file]? # default: on ~/.qldb/command-history
-l --history-limit [0-9]+ # default: 10000

-e --execute ["text"|script-file|STDIN]
```

## Meta Commands

All commands to the shell itself will be prefixed with a backslash (\), e.g:

* `\quit`
    * Quits the shell
* `\help`
    * Prints the lists of meta commands
* `\status`
    * Prints out things like connection status, server ping latency, etc.
* `\metrics [on|off|last]`
    * Determines whether or not metrics will be emitted after the results of a query are shown.
    * last prints out the last known metrics from your previous command.
* `\query-results-format [ion|json|table]`
    * ion Prints the objects from the database as ION documents in text.
    * json Prints the objects from the database as JSON documents in text.
    * table Tabulates the data and prints out the data as rows.
* `\auto-commit`
    * Determines whether each statement will be executed as a transaction or
      not. By default this is off, meaning that statements are all executed as
      individual transactions.
* `\history [limit]`
    * Print out the last limit commands executed.
* `\use [ledger|endpoint] ledger-or-endpoint`
    * Connect to a different endpoint or ledger.
* `\ledger [list|create|delete] [ledger-name]?`
    * Show, create, or delete a ledger.
* `\show-tables`
    * Provide a list of tables in the current ledger.
* `\delimiter delimiter-characters`
    * Specify the delimiter for end of command processing, such as `\n` or `;`.
* `\output [console|file] [outfile]?`
    * Write results either into a file which you specify, or into the console.

## Customizing your display

You can use the `\prompt` command to customize your prompt and this can be saved
in your `~/.qldb/shell.conf` file.

```sh
\prompt qldb
qldb> # this is the default

\prompt ${ledger-name}/${database-name}-${transaction-id}>
PeopleLedger/PeopleDatabase-(a-transaction-id)> ...
```

## Configuration

All possible configuration options are settable on the command line or in a conf
file using the standard MySQL
(https://dev.mysql.com/doc/refman/8.0/en/option-files.html) or Git
(https://git-scm.com/docs/git-config/2.1.4) syntax (TOML
(https://github.com/toml-lang/toml)). The format is as follows:

```toml
[section]
option[ = value]? [# comments]?
[# comments]?

Full options are as follows:

[metrics]
display = [on|off] # default: on

[results]
format = [ion|json\table] # default: ion

[transactions]
auto-commit = [on|off] # default: on

[connection]
endpoint = default-endpoint # default: taken from credentials
ledger = default-ledger # default: none

[interface]
prompt = your-prompt-syntax # default: qldb>
termintor-required = [true|false] # default: true
delimiter = your-statement-delimiter # default: ;
output = [console|file] [outfile]? # default: console

[history]
record-commands = [on|off] [file]? # default: on ~/.qldb/command-history
limit = [0-9]+ # default: 10000
```

### Comments

Any statement that starts with a hash (`#`) will be ignored as a comment.

## Command interface

The way to indicate to the shell that you are done with a command is by using
CTRL-LF (`\n` or `\r\n`). Some SQL data miners using shells regularly type out
long commands and separating this over multiple lines is natural. For these
users they can change their terminator to be required and set the command
delimiter to be semi-colon (;) as follows:

```
\terminator-required true
\delimiter ;
```

`\terminator-required` indicates that pressing the enter key at the end of a
line of input will not execute the command by itself. If they did that some
sample output would look like this:

```
qldb> select * from person
    -> ...
```

Alternately, if you end your statement with a semi-colon (`;`) you will execute
the statement:

```
qldb> \query-results-format table
qldb> select * from Person;
+------+------+-------------+
| name | age  | age.measure |
+------+------+-------------+
| Ian  |  44  | years       |
+------+------+-------------+
1 document in bag (read-ios: 1, server-time: 3ms, total-time: 4ms)
```

We support readline, so things like up-arrow, i-search, CTRL-D work as expected.

## Transactions

Following MySQL's lead, starting a transaction uses the following statement
(which is not currently reserved in PartiQL):

```
qldb> \query-results-format table
qldb> start transaction;
+------------------+
| transactionId    |
+------------------+
| a-transaction-id |
+------------------+
1 document in set (total-time: 1.03ms)
qldb> select * from Person;
+------+------+
| name | age  |
+------+------+
| Ian  |  44  |
+------+------+
1 document in bag (read-ios: 1, server-time: 3.14ms, total-time: 3.59ms)
qldb> commit;
transaction a-transaction-id committed (read-ios: 1, server-time: 1.31ms, total-time: 1.4ms)
```

In MySQL begin is an alias for start transaction but it is not recommended as
start transaction is standard SQL syntax. We will not implement a begin alias
for starting a transaction unless a customer asks for it to avoid possible
complications with the begin...end compound statement found in PL/ and T-SQL.

## Query Metrics

By default query metrics are enabled. They print as a simple statement after your results:

```
qldb> select * from Person;
{
  name: "Ian",
  age: 44
}
1 document in bag (read-ios: 1, server-time: 3.14ms, total-time: 3.59ms)
```

You can turn them off with \metrics off.

## Unsupported

We deliberately will not support things from MySQL that we don't feel should come with you to a QLDB world. Such as:

```
select .. into outfile
```

We will not extend PartiQL syntax for CLI specific functionality, we intend to
use commands for that.
