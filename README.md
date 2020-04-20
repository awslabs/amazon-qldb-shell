# Amazon QLDB Shell

This tool provides an interface to send PartiQL statements to [Amazon Quantum Ledger Database (QLDB)](https://aws.amazon.com/qldb/). 
 This tool is not intended to be incorporated into an application or adopted for production purposes. 
 The objective of the tool is to give developers, devops, database administrators, and anyone else interested the opportunity for rapid experimentation with QLDB and [PartiQL](https://docs.aws.amazon.com/qldb/latest/developerguide/ql-reference.html). 
## Prerequisites

### Basic Configuration

See [Accessing Amazon QLDB](https://docs.aws.amazon.com/qldb/latest/developerguide/accessing.html) for information on connecting to AWS.

### Python 3.4 or later

The driver requires Python 3.4 or later. Please see the link below for more detail to install Python:

* [Python Installation](https://www.python.org/downloads/)


### Getting Started
Install the QLDB Shell using pip:

```pip3 install qldbshell```

### Invocation
The shell can then be invoked by using the following command:

```shell
$ qldbshell --region <region_code> --ledger <ledger_name>
```
An example region code that can be used is us-east-1.
The currently avaiable regions are addressed in the [QLDB General Reference](https://docs.aws.amazon.com/general/latest/gr/qldb.html) page.
By default, the shell will use the credentials specified as environment variables and then in the default profile mentioned in `~/.aws/credentials/` (default location set in the AWS_SHARED_CREDENTIALS_FILE environment variable) and then the default profile in `~/.aws/config` (default location set in AWS_CONFIG_FILE environment variable). 
Various optional arguments can be added to override the profile, endpoints, and region used. To view the arguments, execute the following:

```shell
$ qldbshell --help
```

### Example Usage
Assuming that the ledger, "test-ledger" has already been created:
```shell
$ qldbshell --region us-east-1 --ledger test-ledger
qldbshell> CREATE TABLE TestTable
qldbshell> INSERT INTO TestTable `{"Name": "John Doe"}` 
qldbshell> SELECT * FROM TestTable
qldbshell> exit
```
We use backticks in the example above since we use are using Ion literals. For more on querying Ion literals, go [here](https://docs.aws.amazon.com/qldb/latest/developerguide/ql-reference.query.html).
Each statement between connect and disconnect is considered as a transaction.

### See also

1. Amazon QLDB accepts and stores [Amazon ION](http://amzn.github.io/ion-docs/) Documents. Amazon Ion is a richly-typed, self-describing, hierarchical data serialization format offering interchangeable binary and text representations. For more information read the [ION docs](https://readthedocs.org/projects/ion-python/).
2. Amazon QLDB supports the [PartiQL](https://partiql.org/) query language. PartiQL provides SQL-compatible query access across multiple data stores containing structured data, semistructured data, and nested data. For more information read the [PartiQL docs](https://partiql.org/docs.html).
3. We use backticks in our example since we use are using Ion literals. For more on querying Ion with PartiQL, go [here](https://docs.aws.amazon.com/qldb/latest/developerguide/ql-reference.query.html).

## Development
### Setting up the Shell

After cloning this repository, activate a virtual environment and install the package by running:
```shell
$ virtualenv venv
...
$ . venv/bin/activate
$ pip install -r requirements.txt
$ pip install -e .
```

## License

This tool is licensed under the Apache 2.0 License.
