from setuptools import setup, find_packages
import os

repo_base_dir = os.path.abspath(os.path.dirname(__file__))

package_about = {}
with open(os.path.join(repo_base_dir, "auditorclient", "__about__.py")) as about_file:
    exec(about_file.read(), package_about)

with open(os.path.join(repo_base_dir, "README.md"), "r") as read_me:
    long_description = read_me.read()

TESTS_REQUIRE = ["flake8", "aioresponses"]


setup(
    name=package_about["__package__"],
    version=package_about["__version__"],
    description=package_about["__summary__"],
    long_description=long_description,
    long_description_content_type="text/markdown",
    url=package_about["__url__"],
    author=package_about["__author__"],
    author_email=package_about["__email__"],
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Intended Audience :: System Administrators",
        "Intended Audience :: Information Technology",
        "Intended Audience :: Science/Research",
        "Operating System :: OS Independent",
        "Topic :: System :: Distributed Computing",
        "Topic :: Scientific/Engineering :: Physics",
        "Topic :: Utilities",
        "Framework :: AsyncIO",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
    ],
    keywords=package_about["__keywords__"],
    packages=find_packages(exclude=["tests"]),
    install_requires=[
        "aiohttp",
        "aiosqlite",
    ],
    extras_require={
        "docs": ["sphinx", "sphinx_rtd_theme", "sphinxcontrib-contentui"],
        "test": TESTS_REQUIRE,
        "contrib": ["flake8", "flake8-bugbear", "black; implementation_name=='cpython'"]
        + TESTS_REQUIRE,
    },
    tests_require=TESTS_REQUIRE,
    zip_safe=False,
    test_suite="tests",
    project_urls={
        "Bug Reports": "https://github.com/XXX",
        "Source": "https://github.com/XXX",
    },
)
