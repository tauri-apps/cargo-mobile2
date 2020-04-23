#!groovy

projectName = 'cargo-mobile'
branchName = env.BRANCH_NAME
resourceName = "${projectName}_${branchName}_build"

@Library('jenkinsfile-scripts') _

project {
    name 'cargo-mobile'
    branch {
        pattern '^master$'
        pingSlack true
        custom {
            buildStages {
                stage('test') {
                    sh "ci/test.sh"
                }
                stage('test project') {
                    sh "ci/test_project.sh"
                }
                stage('install') {
                    sh "ci/install.sh"
                }
            }
        }
    }
    branch {
        pattern '.*'
        custom {
            buildStages {
                stage('test') {
                    sh "ci/test.sh"
                }
                stage('test project') {
                    sh "ci/test_project.sh"
                }
            }
        }
    }
}
