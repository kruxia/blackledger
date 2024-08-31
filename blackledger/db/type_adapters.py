from psycopg import adapt, adapters

from blackledger import types


class NormalDumper(adapt.Dumper):
    def dump(self, obj):
        return obj.name.encode()


adapters.register_dumper(types.Normal, NormalDumper)
