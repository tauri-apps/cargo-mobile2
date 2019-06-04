#!groovy

projectName = 'cargo-ginit'
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
        citools.failableStage('test', []) {
            sh "ci/test.sh"
        }
        citools.failableStage('install', []) {
            sh "ci/install.sh"
        }
        citools.failableStage('test project', []) {
            sh "ci/test_project.sh"
        }
    }
}
