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

import time
import logging
import functools

def time_this(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        start = time.perf_counter()
        func(*args, **kwargs)
        end = time.perf_counter()
        run_time = end - start
        logging.info(f'({run_time:.4f}s)')
    return wrapper

def single_noun_command(func):
    @functools.wraps(func)
    def wrapper(self, line):
        if len(list(filter(lambda x: x != '', line.split(' ')))) != 1:
            logging.warning(f'This command requires exactly 1 argument. See help <command> for more information. Recieved {line.split(" ")}')
        else:
            func(self, line)
    return wrapper

def zero_noun_command(func):
    @functools.wraps(func)
    def wrapper(self, line):
        if len(list(filter(lambda x: x != '', line.split(' ')))) != 0:
            logging.warning(f'This command requires exactly 0 arguments. See help <command> for more information. Recieved {line.split(" ")}')
        else:
            func(self, line)
    return wrapper
