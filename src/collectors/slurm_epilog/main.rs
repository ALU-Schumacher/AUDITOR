use anyhow::Error;
use auditor::client::AuditorClient;
use auditor::telemetry::{get_subscriber, init_subscriber};
use std::collections::HashMap;
use std::env;
use std::process::Command;

fn get_slurm_job_id() -> Result<String, Error> {
    Ok(env::var("SLURM_JOB_ID")?)
}

fn get_slurm_job_info() -> Result<HashMap<String, String>, Error> {
    Ok(std::str::from_utf8(
        &Command::new("scontrol")
            .arg("show")
            .arg("job")
            .arg(get_slurm_job_id()?)
            .output()?
            .stdout,
    )?
    .split_whitespace()
    .map(|s| {
        let t = s.split('=').take(2).collect::<Vec<_>>();
        (t[0].to_string(), t[1].to_string())
    })
    .collect())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set up logging
    let subscriber = get_subscriber(
        "AUDITOR-slurm-epilog-collector".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let auditor_host = "localhost";
    let auditor_port = 8000;

    let _client = AuditorClient::new(&auditor_host, auditor_port)?;

    let _job_info = get_slurm_job_info();

    //  let output: String = r#"JobId=19771 JobName=arc_pilot
    // UserId=atlpr001(301501) GroupId=atlpr(300001) MCS_label=N/A
    // Priority=59971 Nice=50 Account=atlpr QOS=normal
    // JobState=RUNNING Reason=None Dependency=(null)
    // Requeue=0 Restarts=0 BatchFlag=1 Reboot=0 ExitCode=0:0
    // RunTime=00:15:04 TimeLimit=15:00:00 TimeMin=N/A
    // SubmitTime=2022-05-04T13:56:09 EligibleTime=2022-05-04T13:56:09
    // AccrueTime=2022-05-04T13:56:09
    // StartTime=2022-05-04T13:56:10 EndTime=2022-05-05T04:56:10 Deadline=N/A
    // SuspendTime=None SecsPreSuspend=0 LastSchedEval=2022-05-04T13:56:10 Scheduler=Main
    // Partition=grid_medium_mcore AllocNode:Sid=arc3:1007
    // ReqNodeList=(null) ExcNodeList=(null)
    // NodeList=host-10-18-1-5
    // BatchHost=host-10-18-1-5
    // NumNodes=1 NumCPUs=8 NumTasks=8 CPUs/Task=1 ReqB:S:C:T=0:0:*:*
    // TRES=cpu=8,mem=16000M,node=1,billing=8
    // Socks/Node=* NtasksPerN:B:S:C=8:0:*:* CoreSpec=*
    // MinCPUsNode=8 MinMemoryCPU=2000M MinTmpDiskNode=0
    // Features=(null) DelayBoot=00:00:00
    // OverSubscribe=OK Contiguous=0 Licenses=(null) Network=(null)
    // Command=/tmp/SLURM_job_script.wdil52
    // WorkDir=/pool_home/arc6/session/FhRKDmoWS60nQ0j3QqGs20FqABFKDmABFKDmEYKKDmMBFKDmbGuhXn
    // StdErr=/pool_home/arc6/session/FhRKDmoWS60nQ0j3QqGs20FqABFKDmABFKDmEYKKDmMBFKDmbGuhXn.comment
    // StdIn=/dev/null
    // StdOut=/pool_home/arc6/session/FhRKDmoWS60nQ0j3QqGs20FqABFKDmABFKDmEYKKDmMBFKDmbGuhXn.comment
    // Power="#
    //      .to_string();
    //
    //  println!("{}", output);
    //
    //  let blah: HashMap<&str, &str> = output
    //      .split_whitespace()
    //      .map(|s| {
    //          let t = s.split("=").take(2).collect::<Vec<_>>();
    //          (t[0], t[1])
    //      })
    //      .collect();
    //
    //  println!("{:#?}", blah);

    Ok(())
}
