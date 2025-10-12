from setuptools import setup, find_namespace_packages
import pathlib

here = pathlib.Path(__file__).parent.resolve()

# Get the long description from the README file
long_description = (here / "README.md").read_text(encoding="utf-8")

setup(
    name="stormbird_setup",  
    version="0.1",  
    description="A library for setting up Stormbird simulations in using Python",  
    long_description=long_description, 
    long_description_content_type="text/markdown", 
    url="https://github.com/NTNU-IMT/stormbird",  
    author="Jarle Vinje Kramer", 
    author_email="jarle.a.kramer@ntnu.no",  
    packages=find_namespace_packages(where="src"),
    package_data={"stormbird_setup": ["py.typed"]},
	package_dir={"": "src"},
    python_requires=">=3.10, <4",
    install_requires=[
        'pydantic >= 2.10',
        'mypy >= 1.15',
        'numpy >= 2.2',
    ], 
)