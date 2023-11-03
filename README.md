# pyswc

Python bindings for the [SWC](https://github.com/swc-project/swc) library. Currently support exporting the parsed result in JSON format.

## Usage

Install with `pip install pyswc`.

```python
import pyswc
print(pyswc.parse("console.log('hello world');"))
```

## License

pyswc is distributed under the terms of the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
