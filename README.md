# Amazon QLDB Python CLI

This project provides a basic CLI to abstract over the more tedious aspects of
 experimenting with Amazon QLDB. This project is not intended to be incorporated
 into an application or adopted for production purposes. My intent is to provide
 a window into rapid experimentation with QLDB, allowing you to:

 1. Interact with the customer control plane in a more fluid manner than offered by the AWS SDK.
 2. Execute arbitrary, basic PartiQL statements without directly interacting with database drivers or logging in to the AWS Console.

## Using the Amazon QLDB Python CLI

After cloning this repository, install the package:

```shell
pip install -e .
```

The CLI can then be invoked as follows:

```shell
python pyqldbcli
```

By default, the CLI will use the default and credentials specified in `~/.aws.config/` and `~/.aws/credentials`. Various optional arguments can be added to override the profile, endpoints, and region used. To view the arguments, execute the following:

```shell
pyqldbcli --help
```

### Example Execution

```shell
pyqldbcli
create test-ledger
connect test-ledger
CREATE TABLE TestTable
INSERT INTO TestTable `{"Name": "Bob Smith"}`
SELECT * FROM TestTable
disconnect
quit
```

## TODO

The following tasks should be completed, in roughly the written order:

1. Add some basic tests to prevent changes from breaking the package.
2. Add an 'explicit' mode that allows the user to consciously start the transaction, execute statements, and either commit or abort it.

## License

This project is licensed under the MIT-0 license.
