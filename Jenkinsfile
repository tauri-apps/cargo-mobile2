#!groovy

projectName = 'cargo-mobile'
branchName = env.BRANCH_NAME
resourceName = "${projectName}_${branchName}_build"

milestone()

def citools

lock(resource: resourceName, inversePrecedence: true) {
    stage('git') {
        node {
            // https://github.com/MarkEWaite/jenkins-bugs/blob/JENKINS-35687/Jenkinsfile
            checkout([$class: 'GitSCM',
                      branches: [[name: branchName]],
                      extensions: [[$class: 'GitLFSPull']],
                      userRemoteConfigs: [[url: "git@bitbucket.org:brainium/${projectName}.git"]],
                ]
            )
            sh "ci/git.sh"
            citools = load ('ci-tools/jenkinsfile_functions.groovy')
        }
    }
    citools.parseLog {
        citools.failableStage('test', citools.default_notify_branches) {
            sh "ci/test.sh"
        }
        citools.failableStage('install', citools.default_notify_branches) {
            sh "ci/install.sh ${branchName} ${citools.devBranch}"
        }
        citools.failableStage('test project', citools.default_notify_branches) {
            sh "ci/test_project.sh ${branchName} ${citools.devBranch}"
        }
        stage('success') {
            node {
                citools.backToNormalGuarded(citools.default_notify_branches)
            }
        }
    }
}
