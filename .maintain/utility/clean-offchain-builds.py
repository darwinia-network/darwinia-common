from glob import iglob
from os.path import abspath, dirname
from pathlib import Path
from shutil import rmtree

for mode in ['debug', 'release']:
    for pattern in ['*eth_offchain*']:
        for path in iglob(''.join([dirname(dirname(dirname(abspath(__file__)))), '/target/', mode, '/**/', pattern]), recursive=True):
            print('removed:', path)
            path = Path(path)
            if path.is_dir():
                rmtree(path)
            elif path.is_file:
                path.unlink()
