#!/usr/bin/env python

# Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: MIT-0
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of this
# software and associated documentation files (the "Software"), to deal in the Software
# without restriction, including without limitation the rights to use, copy, modify,
# merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
# permit persons to whom the Software is furnished to do so.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
# INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
# PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
# HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
# OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
# SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

import argparse
import logging
import sys

from botocore.exceptions import ClientError, EndpointConnectionError

from .qldb_cli import QldbCli


def main():
    parser = argparse.ArgumentParser(
        description="A CLI wrapper around the AWS SDK for QLDB and `pyqldb`, the Amazon QLDB Driver for Python.",
        epilog="As an alternative to the commandline, params can be placed in a file, one per line, and specified on the commandline like '%(prog)s @params.conf'.",
        fromfile_prefix_chars='@')
    parser.add_argument(
        "-v",
        "--verbose",
        help="increase output verbosity",
        action="store_true")
    parser.add_argument(
        "-r",
        "--region",
        help="AWS Region to use for credentials and/or endpoint formation, e.g. us-east-1",
        action="store",
    )
    parser.add_argument(
        "-c",
        "--qldb-endpoint",
        help="Endpoint to use for the qldb API",
        action="store",
        dest="qldb_endpoint"
    )
    parser.add_argument(
        "-s",
        "--qldb-session-endpoint",
        help="Endpoint to use for the qldb-session API",
        action="store",
        dest="qldb_session_endpoint"
    )
    parser.add_argument(
        "-l",
        "--ledger",
        help="Name of a ledger to initially connec to",
        action="store",
    )
    parser.add_argument(
        "-p",
        "--profile",
        help="Name of a profile whose credentials we should use",
        action="store",
    )
    args = parser.parse_args()

  # Setup logging
    if args.verbose:
        loglevel = logging.DEBUG
    else:
        loglevel = logging.INFO
    logging.basicConfig(format="%(levelname)s: %(message)s", level=loglevel)
    cli = QldbCli(args.profile, args.qldb_endpoint,
                  qldb_session_endpoint=args.qldb_session_endpoint, region=args.region, ledger=args.ledger)
    cli.cmdloop()


# Standard boilerplate to call the main() function to begin
# the program.
if __name__ == '__main__':
    main()
