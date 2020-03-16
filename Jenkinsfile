#!groovy

@Library('jenkinsfile-scripts@refactor') _


project {
    name 'ginit'
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
