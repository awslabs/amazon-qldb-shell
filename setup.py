# Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
#
# Licensed under the Apache License, Version 2.0 (the "License").
# You may not use this file except in compliance with the License.
# A copy of the License is located at
#
# http://www.apache.org/licenses/LICENSE-2.0
#  
# or in the "license" file accompanying this file. This file is distributed 
# on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either 
# express or implied. See the License for the specific language governing 
# permissions and limitations under the License.

from setuptools import setup
from pyqldbcli import version

requires = ['pyqldb>=1.0.0rc2',
            'boto3>=1.9.237',
            'amazon.ion>=0.5.0']

setup(
    name='pyqldbcli',
    version=version,
    packages=['pyqldbcli'],
    description='A basic CLI for interacting with Amazon QLDB',
    long_description=open('README.md').read(),
    long_description_content_type='text/markdown',
    author='Amazon Web Services',
    install_requires=requires,
    license="Apache License 2.0",
    entry_points={
        'console_scripts': [
            'pyqldbcli = pyqldbcli.__main__:main'
        ]
    })