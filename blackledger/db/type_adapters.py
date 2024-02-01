from psycopg import adapt, adapters

from blackledger.domain import types


class UUIDLoader(adapt.Loader):
    def load(self, data):
        return types.ID.from_uuid(bytes(data).decode())


adapters.register_loader("uuid", UUIDLoader)


class IDDumper(adapt.Dumper):
    oid = adapters.types["uuid"].oid

    def dump(self, obj):
        return str(obj.to_uuid()).encode()


adapters.register_dumper(types.ID, IDDumper)


class NormalDumper(adapt.Dumper):
    def dump(self, obj):
        return obj.name.encode()


adapters.register_dumper(types.NormalType, NormalDumper)
