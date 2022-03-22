from unittest import TestCase
from auditorclient.record import Record, Components
from auditorclient.errors import InsufficientParametersError


class TestComponents(TestCase):
    def test_components(self):
        comp = Components()
        self.assertEqual(comp._components, [])
        self.assertEqual(comp.get(), [])
        comp.add_component("blaah", 1, 1.2)
        self.assertEqual(comp.get(), [{"name": "blaah", "amount": 1, "factor": 1.2}])
        comp.add_component("blubb", 2, 5)
        self.assertEqual(
            comp.get(),
            [
                {"name": "blaah", "amount": 1, "factor": 1.2},
                {"name": "blubb", "amount": 2, "factor": 5},
            ],
        )
        self.assertEqual(comp.__str__(), comp._components.__str__())


class TestRecord(TestCase):
    def test_record(self):
        with self.assertRaises(InsufficientParametersError):
            record = Record()

        record = Record(
            "record", "site", "user", "group", Components().add_component("comp1", 1, 2.0)
        )
        self.assertEqual(record.record_id(), "record")
        self.assertEqual(record.site_id(), "site")
        self.assertEqual(
            record.as_dict(),
            {
                "record_id": "record",
                "site_id": "site",
                "user_id": "user",
                "group_id": "group",
                "components": [{"name": "comp1", "amount": 1, "factor": 2.0}],
                "start_time": None,
                "stop_time": None,
            },
        )

        record.with_start_time("time1")
        self.assertEqual(
            record.as_dict(),
            {
                "record_id": "record",
                "site_id": "site",
                "user_id": "user",
                "group_id": "group",
                "components": [{"name": "comp1", "amount": 1, "factor": 2.0}],
                "start_time": "time1",
                "stop_time": None,
            },
        )

        record.with_stop_time("time2")
        self.assertEqual(
            record.as_dict(),
            {
                "record_id": "record",
                "site_id": "site",
                "user_id": "user",
                "group_id": "group",
                "components": [{"name": "comp1", "amount": 1, "factor": 2.0}],
                "start_time": "time1",
                "stop_time": "time2",
            },
        )
        self.assertEqual(record.__str__(), record.as_dict().__str__())
        self.assertEqual(
            record.as_json(),
            '{"record_id": "record", "site_id": "site", "user_id": "user", '
            + '"group_id": "group", '
            + '"components": [{"name": "comp1", "amount": 1, "factor": 2.0}], '
            + '"start_time": "time1", "stop_time": "time2"}',
        )

    def test_record_from_json(self):
        record = Record(
            json_str='{"record_id": "record", "site_id": "site", "user_id": "user", '
            + '"group_id": "group", '
            + '"components": [{"name": "comp1", "amount": 1, "factor": 2.0}], '
            + '"start_time": "time1", "stop_time": "time2"}',
        )
        self.assertEqual(
            record.as_dict(),
            {
                "record_id": "record",
                "site_id": "site",
                "user_id": "user",
                "group_id": "group",
                "components": [{"name": "comp1", "amount": 1, "factor": 2.0}],
                "start_time": "time1",
                "stop_time": "time2",
            },
        )
