
import os
import pysys
from pysys.basetest import BaseTest

import sys

from environment_c8y import EnvironmentC8y

import time


class Environment_roundtrip_c8y(EnvironmentC8y):
    def setup(self):
        super().setup()
        self.log.debug("C8y Roundtrip Setup")
        self.addCleanupFunction(self.myenvroundtripcleanup)

    def execute(self):
        super().execute()
        self.log.debug("C8y Roundtrip Execute")

        self.script = self.project.tebasedir + "ci/roundtrip_local_to_c8y.py"
        self.cmd = os.path.expanduser(self.script)

        # bad hack to wait until the receive window is empty again
        time.sleep(int(self.timeslot))

        sub = self.startPython(
            environs={"C8YPASS":self.project.c8ypass},
            arguments=[
                self.cmd,
                "-m",
                self.style,
                "-pub",
                self.project.exampledir,
                "-u",
                self.project.username,
                "-t",
                self.project.tenant,
                "-id",
                self.project.deviceid,
                "-o",
                self.timeslot,
                "-d",
                self.delay,
                "-s",
                self.samples,
            ],
            stdouterr="stdout",
        )

    def validate(self):
        super().validate()
        self.log.debug("C8y Roundtrip Validate")
        self.assertGrep("stdout.out", expr="Data verification PASSED", contains=True)
        self.assertGrep(
            "stdout.out", expr="Timestamp verification PASSED", contains=True
        )

    def myenvroundtripcleanup(self):
        self.log.debug("C8y Roundtrip MyCleanup")

